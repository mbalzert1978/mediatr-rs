//! Tests für den Abschnitt „Mediator — Fassade und DI“ in `TESTLIST.md`.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, mpsc};

use mediatr_rs::{Handler, Mediator, RequestHandler, SendError, Sender, WeakMediator};

mod mediator_fixtures;

use mediator_fixtures::{
    Confirmation, CustomerDirectory, CustomerError, FixedCustomer, OrderError, PlaceOrder,
    PlaceOrderHandler, WarehouseOrderHandler, ada,
};

#[tokio::test]
async fn handler_processes_data_fetched_through_injected_sender() {
    let mediator = Mediator::builder()
        .register(Handler::instance(CustomerDirectory {
            customers: vec![ada()],
        }))
        .register(Handler::transient(|sender| PlaceOrderHandler { sender }))
        .build();

    let confirmation = mediator
        .send(PlaceOrder {
            customer_id: 7,
            amount: 100,
        })
        .await
        .unwrap();

    assert_eq!(
        confirmation,
        Confirmation {
            customer: "Ada".to_owned(),
            total: 90,
        }
    );
}

#[tokio::test]
async fn handler_wraps_failed_inner_request_in_own_error() {
    let mediator = Mediator::builder()
        .register(Handler::instance(CustomerDirectory { customers: vec![] }))
        .register(Handler::transient(|sender| PlaceOrderHandler { sender }))
        .build();

    let result = mediator
        .send(PlaceOrder {
            customer_id: 7,
            amount: 100,
        })
        .await;

    assert!(matches!(
        result,
        Err(OrderError::Customer(CustomerError::NotFound))
    ));
}

#[tokio::test]
async fn transient_builds_handler_on_every_send() {
    let built = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&built);
    let mediator = Mediator::builder()
        .register(Handler::instance(CustomerDirectory {
            customers: vec![ada()],
        }))
        .register(Handler::transient(move |sender| {
            counter.fetch_add(1, Ordering::SeqCst);
            PlaceOrderHandler { sender }
        }))
        .build();

    for amount in [100, 250] {
        mediator
            .send(PlaceOrder {
                customer_id: 7,
                amount,
            })
            .await
            .unwrap();
    }

    assert_eq!(built.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn singleton_builds_handler_only_on_first_send() {
    let built = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&built);
    let mediator = Mediator::builder()
        .register(Handler::instance(CustomerDirectory {
            customers: vec![ada()],
        }))
        .register(Handler::singleton(move |sender| {
            counter.fetch_add(1, Ordering::SeqCst);
            PlaceOrderHandler { sender }
        }))
        .build();

    for amount in [100, 250] {
        mediator
            .send(PlaceOrder {
                customer_id: 7,
                amount,
            })
            .await
            .unwrap();
    }

    assert_eq!(built.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn singleton_handler_with_sender_is_released_with_mediator() {
    let line_open = Arc::new(AtomicBool::new(true));
    let line = Arc::clone(&line_open);
    let mediator = Mediator::builder()
        .register(Handler::instance(CustomerDirectory {
            customers: vec![ada()],
        }))
        .register(Handler::singleton(move |sender| WarehouseOrderHandler {
            orders: PlaceOrderHandler { sender },
            line_open: Arc::clone(&line),
        }))
        .build();
    mediator
        .send(PlaceOrder {
            customer_id: 7,
            amount: 100,
        })
        .await
        .unwrap();

    drop(mediator);

    assert!(!line_open.load(Ordering::SeqCst));
}

#[tokio::test]
async fn singleton_handler_sending_after_mediator_dropped_gets_mediator_dropped() {
    let order = || PlaceOrder {
        customer_id: 7,
        amount: 100,
    };
    let (keep, handed_out) = mpsc::channel();
    let mediator = Mediator::builder()
        .register(Handler::instance(CustomerDirectory {
            customers: vec![ada()],
        }))
        .register(Handler::singleton(move |sender: WeakMediator| {
            keep.send(sender.clone()).unwrap();
            PlaceOrderHandler { sender }
        }))
        .build();
    mediator.send(order()).await.unwrap();
    let orders = PlaceOrderHandler {
        sender: handed_out.try_recv().unwrap(),
    };
    drop(mediator);

    let result = orders.handle(order()).await;

    assert!(matches!(
        result,
        Err(OrderError::Customer(CustomerError::Undeliverable(
            SendError::MediatorDropped
        )))
    ));
}

#[tokio::test]
async fn handler_processes_response_of_fake_sender() {
    let handler = PlaceOrderHandler {
        sender: FixedCustomer(ada()),
    };

    let confirmation = handler
        .handle(PlaceOrder {
            customer_id: 7,
            amount: 100,
        })
        .await
        .unwrap();

    assert_eq!(
        confirmation,
        Confirmation {
            customer: "Ada".to_owned(),
            total: 90,
        }
    );
}

#[test]
fn handler_debug_names_handler_type_without_debug() {
    let handler: Handler<_> = Handler::instance(CustomerDirectory {
        customers: vec![ada()],
    });

    let output = format!("{handler:?}");

    assert!(output.contains("CustomerDirectory"), "{output}");
}
