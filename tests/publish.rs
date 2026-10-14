//! Tests für den Abschnitt „Notification — `publish`“ in `TESTLIST.md`.

use std::cell::RefCell;
use std::rc::Rc;

use mediatr_rs::{
    Mediator, Notification, NotificationHandler, Publisher, Request, RequestHandler, SendError,
    Sender,
};

/// Ereignis: Die Bestellung mit dieser Nummer ist eingegangen.
struct OrderPlaced {
    order_id: u32,
}

impl Notification for OrderPlaced {
    type Error = SendError;
}

/// Druckt für jede eingegangene Bestellung ein Versandetikett mit ihrer Nummer.
struct ShippingDesk {
    labels: Rc<RefCell<Vec<u32>>>,
}

impl NotificationHandler<OrderPlaced> for ShippingDesk {
    async fn handle(&self, event: &OrderPlaced) -> Result<(), SendError> {
        self.labels.borrow_mut().push(event.order_id);
        Ok(())
    }
}

#[tokio::test]
async fn publish_runs_registered_handler() {
    let labels = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .subscribe(ShippingDesk {
            labels: Rc::clone(&labels),
        })
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(*labels.borrow(), [42]);
}

#[tokio::test]
async fn publish_without_handler_returns_ok() {
    let mediator = Mediator::builder().build();

    let result = mediator.publish(OrderPlaced { order_id: 42 }).await;

    assert!(matches!(result, Ok(())));
}

/// Reserviert für jede eingegangene Bestellung die Ware und schreibt das ins Protokoll.
struct Warehouse {
    journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<OrderPlaced> for Warehouse {
    async fn handle(&self, event: &OrderPlaced) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Ware reserviert für {}", event.order_id));
        Ok(())
    }
}

/// Stellt für jede eingegangene Bestellung eine Rechnung aus und schreibt das ins Protokoll.
struct Billing {
    journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<OrderPlaced> for Billing {
    async fn handle(&self, event: &OrderPlaced) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Rechnung für {}", event.order_id));
        Ok(())
    }
}

#[tokio::test]
async fn publish_runs_both_handlers_in_registration_order() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .subscribe(Warehouse {
            journal: Rc::clone(&journal),
        })
        .subscribe(Billing {
            journal: Rc::clone(&journal),
        })
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(
        *journal.borrow(),
        ["Ware reserviert für 42", "Rechnung für 42"]
    );
}

/// Ereignis: Die Bestellung mit dieser Nummer ist storniert.
struct OrderCancelled {
    order_id: u32,
}

#[derive(Debug, PartialEq)]
enum CancellationError {
    Undeliverable(SendError),
    PaymentProviderDown,
}

impl From<SendError> for CancellationError {
    fn from(error: SendError) -> Self {
        Self::Undeliverable(error)
    }
}

impl Notification for OrderCancelled {
    type Error = CancellationError;
}

/// Zahlt den Betrag einer stornierten Bestellung zurück; der Zahlungsdienst ist gerade nicht erreichbar.
struct Refund;

impl NotificationHandler<OrderCancelled> for Refund {
    async fn handle(&self, _event: &OrderCancelled) -> Result<(), CancellationError> {
        Err(CancellationError::PaymentProviderDown)
    }
}

/// Schickt dem Kunden für jede stornierte Bestellung eine Mail mit ihrer Nummer.
struct CancellationMail {
    sent: Rc<RefCell<Vec<u32>>>,
}

impl NotificationHandler<OrderCancelled> for CancellationMail {
    async fn handle(&self, event: &OrderCancelled) -> Result<(), CancellationError> {
        self.sent.borrow_mut().push(event.order_id);
        Ok(())
    }
}

#[tokio::test]
async fn publish_returns_first_handler_error_and_skips_second_handler() {
    let sent = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .subscribe(Refund)
        .subscribe(CancellationMail {
            sent: Rc::clone(&sent),
        })
        .build();

    let result = mediator.publish(OrderCancelled { order_id: 7 }).await;

    assert_eq!(
        (result, sent.borrow().as_slice()),
        (Err(CancellationError::PaymentProviderDown), [].as_slice())
    );
}

#[tokio::test]
async fn publish_runs_handler_type_registered_twice_once_with_first_registration() {
    let front_labels = Rc::new(RefCell::new(Vec::new()));
    let back_labels = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .subscribe(ShippingDesk {
            labels: Rc::clone(&front_labels),
        })
        .subscribe(ShippingDesk {
            labels: Rc::clone(&back_labels),
        })
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(
        (
            front_labels.borrow().as_slice(),
            back_labels.borrow().as_slice()
        ),
        ([42].as_slice(), [].as_slice())
    );
}

#[tokio::test]
async fn publish_reaches_only_handlers_of_its_notification_type() {
    let labels = Rc::new(RefCell::new(Vec::new()));
    let sent = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .subscribe(ShippingDesk {
            labels: Rc::clone(&labels),
        })
        .subscribe(CancellationMail {
            sent: Rc::clone(&sent),
        })
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();
    mediator
        .publish(OrderCancelled { order_id: 7 })
        .await
        .unwrap();

    assert_eq!(
        (labels.borrow().as_slice(), sent.borrow().as_slice()),
        ([42].as_slice(), [7].as_slice())
    );
}

/// Packt für jede eingegangene Bestellung ein Paket; zwischen Beginn und Ende wartet es auf das Lager.
struct Packing {
    journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<OrderPlaced> for Packing {
    async fn handle(&self, event: &OrderPlaced) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Packen beginnt für {}", event.order_id));
        tokio::task::yield_now().await;
        self.journal
            .borrow_mut()
            .push(format!("Paket fertig für {}", event.order_id));
        Ok(())
    }
}

#[tokio::test]
async fn publish_starts_second_handler_after_first_handler_finished_waiting() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .subscribe(Packing {
            journal: Rc::clone(&journal),
        })
        .subscribe(Billing {
            journal: Rc::clone(&journal),
        })
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(
        *journal.borrow(),
        [
            "Packen beginnt für 42",
            "Paket fertig für 42",
            "Rechnung für 42"
        ]
    );
}

/// Schließt die Bestellung mit dieser Nummer ab.
struct Checkout {
    order_id: u32,
}

impl Request for Checkout {
    type Response = ();
    type Error = SendError;
}

/// Schließt Bestellungen ab und meldet sie über den Publisher, den er per Konstruktor bekommt.
struct CheckoutHandler {
    publisher: Mediator,
}

impl RequestHandler<Checkout> for CheckoutHandler {
    async fn handle(&self, checkout: Checkout) -> Result<(), SendError> {
        self.publisher
            .publish(OrderPlaced {
                order_id: checkout.order_id,
            })
            .await
    }
}

#[tokio::test]
async fn request_handler_publishes_through_injected_publisher() {
    let labels = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .transient(|publisher| CheckoutHandler { publisher })
        .subscribe(ShippingDesk {
            labels: Rc::clone(&labels),
        })
        .build();

    mediator.send(Checkout { order_id: 42 }).await.unwrap();

    assert_eq!(*labels.borrow(), [42]);
}
