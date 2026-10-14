//! Tests für den Abschnitt „Notification — `publish`“ in `TESTLIST.md`.

use std::cell::{Cell, OnceCell, RefCell};
use std::error::Error;
use std::fmt;
use std::rc::Rc;

use mediatr_rs::{
    Handler, Mediator, Notification, NotificationHandler, Publisher, Request, RequestHandler,
    SendError, Sender, WeakMediator,
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
        .subscribe(Handler::instance(ShippingDesk {
            labels: Rc::clone(&labels),
        }))
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
        .subscribe(Handler::instance(Warehouse {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(Billing {
            journal: Rc::clone(&journal),
        }))
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

impl fmt::Display for CancellationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Undeliverable(error) => write!(f, "Storno nicht zustellbar: {error}"),
            Self::PaymentProviderDown => f.write_str("Zahlungsdienst nicht erreichbar"),
        }
    }
}

impl Error for CancellationError {}

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
        .subscribe(Handler::instance(Refund))
        .subscribe(Handler::instance(CancellationMail {
            sent: Rc::clone(&sent),
        }))
        .build();

    let result = mediator.publish(OrderCancelled { order_id: 7 }).await;

    assert_eq!(
        (result, sent.borrow().as_slice()),
        (Err(CancellationError::PaymentProviderDown), [].as_slice())
    );
}

/// Zwei Versandstellen, die ihre Etiketten getrennt sammeln: Daran sieht ein Test, welche von zwei Registrierungen desselben Handler-Typs gelaufen ist.
#[derive(Clone, Default)]
struct TwoDesks {
    front: Rc<RefCell<Vec<u32>>>,
    back: Rc<RefCell<Vec<u32>>>,
}

impl TwoDesks {
    /// Die vordere Versandstelle.
    fn front(&self) -> ShippingDesk {
        ShippingDesk {
            labels: Rc::clone(&self.front),
        }
    }

    /// Die hintere Versandstelle.
    fn back(&self) -> ShippingDesk {
        ShippingDesk {
            labels: Rc::clone(&self.back),
        }
    }

    /// Die Etiketten der vorderen und der hinteren Versandstelle.
    fn labels(&self) -> (Vec<u32>, Vec<u32>) {
        (self.front.borrow().clone(), self.back.borrow().clone())
    }
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
    let labels = Rc::new(RefCell::new(Vec::new()));
    let sent = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .subscribe(Handler::instance(ShippingDesk {
            labels: Rc::clone(&labels),
        }))
        .subscribe(Handler::instance(CancellationMail {
            sent: Rc::clone(&sent),
        }))
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
        .subscribe(Handler::instance(Packing {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(Billing {
            journal: Rc::clone(&journal),
        }))
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
struct CheckoutHandler<P> {
    publisher: P,
}

impl<P: Publisher<OrderPlaced>> RequestHandler<Checkout> for CheckoutHandler<P> {
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
        .register(Handler::transient(|publisher| CheckoutHandler {
            publisher,
        }))
        .subscribe(Handler::instance(ShippingDesk {
            labels: Rc::clone(&labels),
        }))
        .build();

    mediator.send(Checkout { order_id: 42 }).await.unwrap();

    assert_eq!(*labels.borrow(), [42]);
}

/// Attrappe statt des Mediators: merkt sich die Nummer jeder gemeldeten Bestellung.
struct RecordedOrders {
    order_ids: Rc<RefCell<Vec<u32>>>,
}

impl Publisher<OrderPlaced> for RecordedOrders {
    async fn publish(&self, event: OrderPlaced) -> Result<(), SendError> {
        self.order_ids.borrow_mut().push(event.order_id);
        Ok(())
    }
}

#[tokio::test]
async fn handler_publishes_to_fake_publisher() {
    let order_ids = Rc::new(RefCell::new(Vec::new()));
    let handler = CheckoutHandler {
        publisher: RecordedOrders {
            order_ids: Rc::clone(&order_ids),
        },
    };

    handler.handle(Checkout { order_id: 42 }).await.unwrap();

    assert_eq!(*order_ids.borrow(), [42]);
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
            labels: Rc::new(RefCell::new(Vec::new())),
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
    let labels = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .register(Handler::singleton(|publisher: WeakMediator| {
            CheckoutHandler { publisher }
        }))
        .subscribe(Handler::instance(ShippingDesk {
            labels: Rc::clone(&labels),
        }))
        .build();

    mediator.send(Checkout { order_id: 42 }).await.unwrap();

    assert_eq!(*labels.borrow(), [42]);
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
    let builds = Rc::new(Cell::new(0_u32));
    let counter = Rc::clone(&builds);
    let labels = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .subscribe(Handler::transient(move |_publisher: Mediator| {
            counter.set(counter.get().saturating_add(1));
            ShippingDesk {
                labels: Rc::clone(&labels),
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

    assert_eq!(builds.get(), 2);
}

/// Ereignis: Die Bestellung mit dieser Nummer steht unter Betrugsverdacht.
struct FraudSuspected {
    order_id: u32,
}

impl Notification for FraudSuspected {
    type Error = SendError;
}

/// Prüft jede eingegangene Bestellung und meldet gesperrte Nummern über den Mediator, den sie per Konstruktor bekommt.
struct FraudCheck {
    publisher: Mediator,
    blocked: Vec<u32>,
}

impl NotificationHandler<OrderPlaced> for FraudCheck {
    async fn handle(&self, event: &OrderPlaced) -> Result<(), SendError> {
        if !self.blocked.contains(&event.order_id) {
            return Ok(());
        }
        self.publisher
            .publish(FraudSuspected {
                order_id: event.order_id,
            })
            .await
    }
}

/// Legt für jeden Betrugsverdacht einen Fall mit der Bestellnummer an.
struct FraudDesk {
    cases: Rc<RefCell<Vec<u32>>>,
}

impl NotificationHandler<FraudSuspected> for FraudDesk {
    async fn handle(&self, event: &FraudSuspected) -> Result<(), SendError> {
        self.cases.borrow_mut().push(event.order_id);
        Ok(())
    }
}

#[tokio::test]
async fn subscribe_transient_handler_publishes_through_received_mediator() {
    let cases = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .subscribe(Handler::transient(|publisher| FraudCheck {
            publisher,
            blocked: vec![13],
        }))
        .subscribe(Handler::instance(FraudDesk {
            cases: Rc::clone(&cases),
        }))
        .build();

    mediator
        .publish(OrderPlaced { order_id: 13 })
        .await
        .unwrap();

    assert_eq!(*cases.borrow(), [13]);
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
    let journal = Rc::new(RefCell::new(Vec::new()));
    let (warehouse, billing) = (Rc::clone(&journal), Rc::clone(&journal));
    let mediator = Mediator::builder()
        .subscribe(Handler::transient(move |_publisher: Mediator| Warehouse {
            journal: Rc::clone(&warehouse),
        }))
        .subscribe(Handler::transient(move |_publisher: Mediator| Billing {
            journal: Rc::clone(&billing),
        }))
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

/// Sperrt den Kunden der Bestellung mit dieser Nummer.
struct BlockCustomer {
    order_id: u32,
}

impl Request for BlockCustomer {
    type Response = ();
    type Error = SendError;
}

/// Führt die Sperrliste und trägt jede Bestellnummer ein, deren Kunde gesperrt wird.
struct Blocklist {
    order_ids: Rc<RefCell<Vec<u32>>>,
}

impl RequestHandler<BlockCustomer> for Blocklist {
    async fn handle(&self, request: BlockCustomer) -> Result<(), SendError> {
        self.order_ids.borrow_mut().push(request.order_id);
        Ok(())
    }
}

/// Ermittelt bei jedem Betrugsverdacht und lässt den Kunden über den Mediator sperren, den sie per Konstruktor bekommt.
struct FraudInvestigator {
    sender: Mediator,
}

impl NotificationHandler<FraudSuspected> for FraudInvestigator {
    async fn handle(&self, event: &FraudSuspected) -> Result<(), SendError> {
        self.sender
            .send(BlockCustomer {
                order_id: event.order_id,
            })
            .await
    }
}

#[tokio::test]
async fn nested_publish_and_send_through_received_mediators_run_all_handlers_without_panic() {
    let blocked = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .subscribe(Handler::transient(|publisher| FraudCheck {
            publisher,
            blocked: vec![13],
        }))
        .subscribe(Handler::transient(|sender| FraudInvestigator { sender }))
        .register(Handler::instance(Blocklist {
            order_ids: Rc::clone(&blocked),
        }))
        .build();

    let published = mediator.publish(OrderPlaced { order_id: 13 }).await;

    assert_eq!((published, blocked.borrow().clone()), (Ok(()), vec![13]));
}
