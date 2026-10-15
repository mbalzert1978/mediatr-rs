//! Tests für den Abschnitt „Notification — `publish`“ in `TESTLIST.md`.

use std::cell::OnceCell;
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use mediatr_rs::{Handler, Mediator, Publisher, RequestHandler, SendError, Sender, WeakMediator};

mod common;
mod publish_fixtures;

use common::Journal;
use publish_fixtures::{
    Billing, Blocklist, CancellationError, CancellationMail, Checkout, CheckoutHandler, FraudCheck,
    FraudDesk, FraudInvestigator, OrderCancelled, OrderPlaced, Packing, Recorded, RecordedOrders,
    Refund, ShippingDesk, TwoDesks, Warehouse,
};

#[tokio::test]
async fn publish_runs_registered_handler() {
    let labels = Recorded::default();
    let mediator = Mediator::builder()
        .subscribe(Handler::instance(ShippingDesk {
            labels: labels.clone(),
        }))
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(labels.entries(), [42]);
}

#[tokio::test]
async fn publish_without_handler_returns_ok() {
    let mediator = Mediator::builder().build();

    let result = mediator.publish(OrderPlaced { order_id: 42 }).await;

    assert!(matches!(result, Ok(())));
}

#[tokio::test]
async fn publish_runs_both_handlers_in_registration_order() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .subscribe(Handler::instance(Warehouse {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(Billing {
            journal: journal.clone(),
        }))
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(
        journal.entries(),
        ["Ware reserviert für 42", "Rechnung für 42"]
    );
}

#[tokio::test]
async fn publish_returns_first_handler_error_and_skips_second_handler() {
    let sent = Recorded::default();
    let mediator = Mediator::builder()
        .subscribe(Handler::instance(Refund))
        .subscribe(Handler::instance(CancellationMail { sent: sent.clone() }))
        .build();

    let result = mediator.publish(OrderCancelled { order_id: 7 }).await;

    assert_eq!(
        (result, sent.entries()),
        (Err(CancellationError::PaymentProviderDown), vec![])
    );
}

#[tokio::test]
async fn publish_runs_handler_type_registered_twice_once_with_first_registration() {
    let desks = TwoDesks::default();
    let mediator = Mediator::builder()
        .subscribe(Handler::instance(desks.front()))
        .subscribe(Handler::instance(desks.back()))
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(desks.labels(), (vec![42], vec![]));
}

#[tokio::test]
async fn publish_reaches_only_handlers_of_its_notification_type() {
    let labels = Recorded::default();
    let sent = Recorded::default();
    let mediator = Mediator::builder()
        .subscribe(Handler::instance(ShippingDesk {
            labels: labels.clone(),
        }))
        .subscribe(Handler::instance(CancellationMail { sent: sent.clone() }))
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();
    mediator
        .publish(OrderCancelled { order_id: 7 })
        .await
        .unwrap();

    assert_eq!((labels.entries(), sent.entries()), (vec![42], vec![7]));
}

#[tokio::test]
async fn publish_starts_second_handler_after_first_handler_finished_waiting() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .subscribe(Handler::instance(Packing {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(Billing {
            journal: journal.clone(),
        }))
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(
        journal.entries(),
        [
            "Packen beginnt für 42",
            "Paket fertig für 42",
            "Rechnung für 42"
        ]
    );
}

#[tokio::test]
async fn request_handler_publishes_through_injected_publisher() {
    let labels = Recorded::default();
    let mediator = Mediator::builder()
        .register(Handler::transient(|publisher| CheckoutHandler {
            publisher,
        }))
        .subscribe(Handler::instance(ShippingDesk {
            labels: labels.clone(),
        }))
        .build();

    mediator.send(Checkout { order_id: 42 }).await.unwrap();

    assert_eq!(labels.entries(), [42]);
}

#[tokio::test]
async fn handler_publishes_to_fake_publisher() {
    let order_ids = Recorded::default();
    let handler = CheckoutHandler {
        publisher: RecordedOrders {
            order_ids: order_ids.clone(),
        },
    };

    handler.handle(Checkout { order_id: 42 }).await.unwrap();

    assert_eq!(order_ids.entries(), [42]);
}

#[tokio::test]
async fn singleton_handler_publishing_after_mediator_dropped_gets_mediator_dropped() {
    let handed_out = Rc::new(OnceCell::new());
    let keep = Rc::clone(&handed_out);
    let mediator = Mediator::builder()
        .register(Handler::singleton(move |publisher: WeakMediator| {
            keep.get_or_init(|| publisher.clone());
            CheckoutHandler { publisher }
        }))
        .subscribe(Handler::instance(ShippingDesk {
            labels: Recorded::default(),
        }))
        .build();
    mediator.send(Checkout { order_id: 41 }).await.unwrap();
    let checkout = CheckoutHandler {
        publisher: handed_out.get().unwrap().clone(),
    };
    drop(mediator);

    let result = checkout.handle(Checkout { order_id: 42 }).await;

    assert_eq!(result, Err(SendError::MediatorDropped));
}

#[tokio::test]
async fn singleton_handler_publishing_while_mediator_alive_runs_handlers() {
    let labels = Recorded::default();
    let mediator = Mediator::builder()
        .register(Handler::singleton(|publisher: WeakMediator| {
            CheckoutHandler { publisher }
        }))
        .subscribe(Handler::instance(ShippingDesk {
            labels: labels.clone(),
        }))
        .build();

    mediator.send(Checkout { order_id: 42 }).await.unwrap();

    assert_eq!(labels.entries(), [42]);
}

#[tokio::test]
async fn publish_error_propagates_as_boxed_error() {
    let mediator = Mediator::builder()
        .subscribe(Handler::instance(Refund))
        .build();

    let result: Result<(), Box<dyn Error>> =
        async { Ok(mediator.publish(OrderCancelled { order_id: 7 }).await?) }.await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "Zahlungsdienst nicht erreichbar"
    );
}

#[tokio::test]
async fn subscribe_transient_builds_handler_on_every_publish() {
    let builds = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&builds);
    let labels = Recorded::default();
    let mediator = Mediator::builder()
        .subscribe(Handler::transient(move |_publisher: Mediator| {
            counter.fetch_add(1, Ordering::SeqCst);
            ShippingDesk {
                labels: labels.clone(),
            }
        }))
        .build();

    mediator
        .publish(OrderPlaced { order_id: 41 })
        .await
        .unwrap();
    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(builds.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn subscribe_transient_handler_publishes_through_received_mediator() {
    let cases = Recorded::default();
    let mediator = Mediator::builder()
        .subscribe(Handler::transient(|publisher| FraudCheck {
            publisher,
            blocked: vec![13],
        }))
        .subscribe(Handler::instance(FraudDesk {
            cases: cases.clone(),
        }))
        .build();

    mediator
        .publish(OrderPlaced { order_id: 13 })
        .await
        .unwrap();

    assert_eq!(cases.entries(), [13]);
}

#[tokio::test]
async fn publish_runs_handler_type_subscribed_and_subscribed_transient_once_with_first_registration()
 {
    let desks = TwoDesks::default();
    let back = desks.clone();
    let mediator = Mediator::builder()
        .subscribe(Handler::instance(desks.front()))
        .subscribe(Handler::transient(move |_publisher: Mediator| back.back()))
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(desks.labels(), (vec![42], vec![]));
}

#[tokio::test]
async fn publish_runs_handler_type_subscribed_transient_twice_once_with_first_registration() {
    let desks = TwoDesks::default();
    let (first, second) = (desks.clone(), desks.clone());
    let mediator = Mediator::builder()
        .subscribe(Handler::transient(move |_publisher: Mediator| {
            first.front()
        }))
        .subscribe(Handler::transient(move |_publisher: Mediator| {
            second.back()
        }))
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(desks.labels(), (vec![42], vec![]));
}

#[tokio::test]
async fn publish_runs_handler_type_subscribed_transient_then_instance_once_with_transient() {
    let desks = TwoDesks::default();
    let front = desks.clone();
    let mediator = Mediator::builder()
        .subscribe(Handler::transient(move |_publisher: Mediator| {
            front.front()
        }))
        .subscribe(Handler::instance(desks.back()))
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(desks.labels(), (vec![42], vec![]));
}

#[tokio::test]
async fn publish_runs_two_handler_types_subscribed_transient_in_registration_order() {
    let journal = Journal::default();
    let (warehouse, billing) = (journal.clone(), journal.clone());
    let mediator = Mediator::builder()
        .subscribe(Handler::transient(move |_publisher: Mediator| Warehouse {
            journal: warehouse.clone(),
        }))
        .subscribe(Handler::transient(move |_publisher: Mediator| Billing {
            journal: billing.clone(),
        }))
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(
        journal.entries(),
        ["Ware reserviert für 42", "Rechnung für 42"]
    );
}

#[tokio::test]
async fn nested_publish_and_send_through_received_mediators_run_all_handlers_without_panic() {
    let blocked = Recorded::default();
    let mediator = Mediator::builder()
        .subscribe(Handler::transient(|publisher| FraudCheck {
            publisher,
            blocked: vec![13],
        }))
        .subscribe(Handler::transient(|sender| FraudInvestigator { sender }))
        .register(Handler::instance(Blocklist {
            order_ids: blocked.clone(),
        }))
        .build();

    let published = mediator.publish(OrderPlaced { order_id: 13 }).await;

    assert_eq!((published, blocked.entries()), (Ok(()), vec![13]));
}
