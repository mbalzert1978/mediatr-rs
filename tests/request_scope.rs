//! Tests für den Abschnitt „Mediator — Request-Scope“ in `TESTLIST.md`.

use mediatr_rs::{Handler, Mediator, Sender};

mod request_scope_fixtures;

use request_scope_fixtures::{AddToCart, Cart};

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
