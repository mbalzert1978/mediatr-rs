//! Tests für den Abschnitt „Mediator — Request-Scope“ in `TESTLIST.md`.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, mpsc};

use futures_util::StreamExt;
use mediatr_rs::{
    Handler, Mediator, Publisher, Registration, RequestHandler, SendError, Sender, StreamSender,
    WeakMediator,
};

mod common;
mod request_scope_fixtures;

use common::Journal;
use request_scope_fixtures::{
    AddGiftWrap, AddOrderLine, AddToCart, Cart, CartAudit, CartCheckedOut, CartContents,
    CheckoutCart, CountOrderLines, GIFT_WRAP_PRICE, GiftWrapHandler, ListCartItems,
    OrderLineCommitter, OrderLineCounter, OrderLineRecorder, ReceiptPrinter, UnitOfWork,
    cart_mediator,
};

#[tokio::test]
async fn scoped_cart_sums_items_added_within_same_scope() {
    let mediator = cart_mediator();
    let scope = mediator.scope();

    scope.send(AddToCart { price: 300 }).await.unwrap();
    let total = scope.send(AddToCart { price: 120 }).await;

    assert_eq!(total, Ok(420));
}

#[tokio::test]
async fn scoped_cart_starts_empty_in_each_new_scope() {
    let mediator = cart_mediator();

    let (morning, evening) = (mediator.scope(), mediator.scope());

    let first = morning.send(AddToCart { price: 300 }).await;
    let second = evening.send(AddToCart { price: 120 }).await;

    assert_eq!((first, second), (Ok(300), Ok(120)));
}

#[tokio::test]
async fn transient_handler_reaches_scoped_cart_of_its_scope() {
    let mediator = Mediator::builder()
        .register(Handler::scoped(|_| Cart::default()))
        .register(Handler::transient(|sender| GiftWrapHandler { sender }))
        .build();
    let scope = mediator.scope();
    scope.send(AddToCart { price: 300 }).await.unwrap();

    let total = scope.send(AddGiftWrap).await;

    assert_eq!(total, Ok(300 + GIFT_WRAP_PRICE));
}

#[tokio::test]
async fn scoped_cart_without_scope_gets_no_scope() {
    let mediator = cart_mediator();

    let total = mediator.send(AddToCart { price: 300 }).await;

    assert_eq!(total, Err(SendError::NoScope));
}

#[tokio::test]
async fn singleton_handler_sending_to_scoped_cart_gets_no_scope() {
    let mediator = Mediator::builder()
        .register(Handler::scoped(|_| Cart::default()))
        .register(Handler::singleton(|sender| GiftWrapHandler { sender }))
        .build();

    let total = mediator.scope().send(AddGiftWrap).await;

    assert_eq!(total, Err(SendError::NoScope));
}

#[tokio::test]
async fn singleton_handler_is_built_once_across_scopes_and_mediator() {
    let built = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&built);
    let mediator = Mediator::builder()
        .register(Handler::singleton(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
            Cart::default()
        }))
        .build();
    let (morning, evening) = (mediator.scope(), mediator.scope());

    for sender in [&morning, &evening, &mediator] {
        sender.send(AddToCart { price: 300 }).await.unwrap();
    }

    assert_eq!(built.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn behavior_runs_around_scoped_cart_in_scope() {
    let seen = Arc::new(AtomicU32::new(0));
    let mediator = Mediator::builder()
        .behavior(CartAudit {
            seen: Arc::clone(&seen),
        })
        .register(Handler::scoped(|_| Cart::default()))
        .build();

    let total = mediator.scope().send(AddToCart { price: 300 }).await;

    assert_eq!((total, seen.load(Ordering::SeqCst)), (Ok(300), 1));
}

#[tokio::test]
async fn scoped_cart_is_released_with_its_scope() {
    let till = Arc::new(AtomicBool::new(true));
    let till_open = Arc::clone(&till);
    let mediator = Mediator::builder()
        .register(Handler::scoped(move |_| CheckoutCart {
            cart: Cart::default(),
            till_open: Arc::clone(&till_open),
        }))
        .build();
    let scope = mediator.scope();
    scope.send(AddToCart { price: 300 }).await.unwrap();

    drop(scope);

    assert!(!till.load(Ordering::SeqCst));
}

#[tokio::test]
async fn scoped_handler_sending_after_scope_dropped_gets_mediator_dropped() {
    let (keep, handed_out) = mpsc::channel();
    let mediator = Mediator::builder()
        .register(Handler::scoped(|_| Cart::default()))
        .register(Handler::scoped(move |sender: WeakMediator| {
            keep.send(sender.clone()).unwrap();
            GiftWrapHandler { sender }
        }))
        .build();
    let scope = mediator.scope();
    scope.send(AddGiftWrap).await.unwrap();
    let gift_wrap = GiftWrapHandler {
        sender: handed_out.try_recv().unwrap(),
    };
    drop(scope);

    let total = gift_wrap.handle(AddGiftWrap).await;

    assert_eq!(total, Err(SendError::MediatorDropped));
}

#[tokio::test]
async fn scoped_notification_handler_runs_when_published_through_scope() {
    let journal = Journal::default();
    let receipts = journal.clone();
    let mediator = Mediator::builder()
        .subscribe(Handler::scoped(move |_| ReceiptPrinter {
            journal: receipts.clone(),
        }))
        .build();

    mediator
        .scope()
        .publish(CartCheckedOut { total: 420 })
        .await
        .unwrap();

    assert_eq!(journal.entries(), ["Beleg über 420"]);
}

#[tokio::test]
async fn publish_to_scoped_handler_without_scope_gets_no_scope() {
    let mediator = Mediator::builder()
        .subscribe(Handler::scoped(|_| ReceiptPrinter {
            journal: Journal::default(),
        }))
        .build();

    let result = mediator.publish(CartCheckedOut { total: 420 }).await;

    assert_eq!(result, Err(SendError::NoScope));
}

#[tokio::test]
async fn scoped_stream_handler_yields_items_through_scope() {
    let mediator = Mediator::builder()
        .register_stream(Handler::scoped(|_| CartContents {
            prices: vec![300, 120],
        }))
        .build();

    let prices: Vec<_> = mediator
        .scope()
        .create_stream(ListCartItems)
        .collect()
        .await;

    assert_eq!(prices, vec![Ok(300), Ok(120)]);
}

#[tokio::test]
async fn stream_from_scoped_handler_without_scope_yields_no_scope() {
    let mediator = Mediator::builder()
        .register_stream(Handler::scoped(|_| CartContents {
            prices: vec![300, 120],
        }))
        .build();

    let prices: Vec<_> = mediator.create_stream(ListCartItems).collect().await;

    assert_eq!(prices, vec![Err(SendError::NoScope)]);
}

#[tokio::test]
async fn scoped_service_is_shared_by_handlers_within_same_scope() {
    let mediator = Mediator::builder()
        .service(Registration::scoped(|_| Ok(UnitOfWork::default())))
        .register(Handler::scoped_from(|_, provider| {
            Ok(OrderLineRecorder {
                unit_of_work: provider.get()?,
            })
        }))
        .register(Handler::scoped_from(|_, provider| {
            Ok(OrderLineCounter {
                unit_of_work: provider.get()?,
            })
        }))
        .build();
    let scope = mediator.scope();
    scope.send(AddOrderLine).await.unwrap();

    let lines = scope.send(CountOrderLines).await;

    assert_eq!(lines, Ok(1));
}

#[tokio::test]
async fn scoped_service_is_separate_in_each_scope() {
    let mediator = Mediator::builder()
        .service(Registration::scoped(|_| Ok(UnitOfWork::default())))
        .register(Handler::scoped_from(|_, provider| {
            Ok(OrderLineRecorder {
                unit_of_work: provider.get()?,
            })
        }))
        .register(Handler::scoped_from(|_, provider| {
            Ok(OrderLineCounter {
                unit_of_work: provider.get()?,
            })
        }))
        .build();
    mediator.scope().send(AddOrderLine).await.unwrap();

    let lines = mediator.scope().send(CountOrderLines).await;

    assert_eq!(lines, Ok(0));
}

#[tokio::test]
async fn scoped_handler_without_registered_dependency_gets_no_dependency() {
    let mediator = Mediator::builder()
        .register(Handler::scoped_from(|_, provider| {
            Ok(OrderLineRecorder {
                unit_of_work: provider.get()?,
            })
        }))
        .build();

    let result = mediator.scope().send(AddOrderLine).await;

    assert_eq!(result, Err(SendError::NoDependency));
}

#[tokio::test]
async fn publish_to_scoped_handler_without_registered_dependency_gets_no_dependency() {
    let mediator = Mediator::builder()
        .subscribe(Handler::scoped_from(|_, provider| {
            Ok(OrderLineCommitter {
                unit_of_work: provider.get()?,
            })
        }))
        .build();

    let result = mediator
        .scope()
        .publish(CartCheckedOut { total: 420 })
        .await;

    assert_eq!(result, Err(SendError::NoDependency));
}
