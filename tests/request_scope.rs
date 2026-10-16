//! Tests für den Abschnitt „Mediator — Request-Scope“ in `TESTLIST.md`.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use mediatr_rs::{Handler, Mediator, SendError, Sender};

mod request_scope_fixtures;

use request_scope_fixtures::{
    AddGiftWrap, AddToCart, Cart, CartAudit, GIFT_WRAP_PRICE, GiftWrapHandler, cart_mediator,
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
