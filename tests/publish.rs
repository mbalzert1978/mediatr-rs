//! Tests für den Abschnitt „Notification — `publish`“ in `TESTLIST.md`.

use mediatr_rs::{Mediator, Notification, Publisher, SendError};

/// Ereignis: Eine Bestellung ist eingegangen.
struct OrderPlaced;

impl Notification for OrderPlaced {
    type Error = SendError;
}

#[tokio::test]
async fn publish_without_handler_returns_ok() {
    let mediator = Mediator::builder().build();

    let result = mediator.publish(OrderPlaced).await;

    assert!(matches!(result, Ok(())));
}
