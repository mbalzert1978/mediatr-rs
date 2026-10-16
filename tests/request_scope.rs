//! Tests für den Abschnitt „Mediator — Request-Scope“ in `TESTLIST.md`.

use mediatr_rs::{Handler, Mediator, Sender};

mod request_scope_fixtures;

use request_scope_fixtures::{AddGiftWrap, AddToCart, Cart, GIFT_WRAP_PRICE, GiftWrapHandler};

#[tokio::test]
async fn scoped_cart_sums_items_added_within_same_scope() {
    let mediator = Mediator::builder()
        .register(Handler::scoped(|_| Cart::default()))
        .build();
    let scope = mediator.scope();

    scope.send(AddToCart { price: 300 }).await.unwrap();
    let total = scope.send(AddToCart { price: 120 }).await;

    assert_eq!(total, Ok(420));
}

#[tokio::test]
async fn scoped_cart_starts_empty_in_each_new_scope() {
    let mediator = Mediator::builder()
        .register(Handler::scoped(|_| Cart::default()))
        .build();

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
