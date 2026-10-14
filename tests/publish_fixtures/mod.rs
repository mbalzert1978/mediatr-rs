use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::rc::Rc;

use mediatr_rs::{
    Mediator, Notification, NotificationHandler, Publisher, Request, RequestHandler, SendError,
    Sender,
};

/// Ereignis: Die Bestellung mit dieser Nummer ist eingegangen.
pub struct OrderPlaced {
    pub order_id: u32,
}

impl Notification for OrderPlaced {
    type Error = SendError;
}

/// Druckt für jede eingegangene Bestellung ein Versandetikett mit ihrer Nummer.
pub struct ShippingDesk {
    pub labels: Rc<RefCell<Vec<u32>>>,
}

impl NotificationHandler<OrderPlaced> for ShippingDesk {
    async fn handle(&self, event: &OrderPlaced) -> Result<(), SendError> {
        self.labels.borrow_mut().push(event.order_id);
        Ok(())
    }
}

/// Reserviert für jede eingegangene Bestellung die Ware und schreibt das ins Protokoll.
pub struct Warehouse {
    pub journal: Rc<RefCell<Vec<String>>>,
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
pub struct Billing {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<OrderPlaced> for Billing {
    async fn handle(&self, event: &OrderPlaced) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Rechnung für {}", event.order_id));
        Ok(())
    }
}

/// Ereignis: Die Bestellung mit dieser Nummer ist storniert.
pub struct OrderCancelled {
    pub order_id: u32,
}

#[derive(Debug, PartialEq)]
pub enum CancellationError {
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
pub struct Refund;

impl NotificationHandler<OrderCancelled> for Refund {
    async fn handle(&self, _event: &OrderCancelled) -> Result<(), CancellationError> {
        Err(CancellationError::PaymentProviderDown)
    }
}

/// Schickt dem Kunden für jede stornierte Bestellung eine Mail mit ihrer Nummer.
pub struct CancellationMail {
    pub sent: Rc<RefCell<Vec<u32>>>,
}

impl NotificationHandler<OrderCancelled> for CancellationMail {
    async fn handle(&self, event: &OrderCancelled) -> Result<(), CancellationError> {
        self.sent.borrow_mut().push(event.order_id);
        Ok(())
    }
}

/// Zwei Versandstellen, die ihre Etiketten getrennt sammeln: Daran sieht ein Test, welche von zwei Registrierungen desselben Handler-Typs gelaufen ist.
#[derive(Clone, Default)]
pub struct TwoDesks {
    pub front: Rc<RefCell<Vec<u32>>>,
    pub back: Rc<RefCell<Vec<u32>>>,
}

impl TwoDesks {
    /// Die vordere Versandstelle.
    pub fn front(&self) -> ShippingDesk {
        ShippingDesk {
            labels: Rc::clone(&self.front),
        }
    }

    /// Die hintere Versandstelle.
    pub fn back(&self) -> ShippingDesk {
        ShippingDesk {
            labels: Rc::clone(&self.back),
        }
    }

    /// Die Etiketten der vorderen und der hinteren Versandstelle.
    pub fn labels(&self) -> (Vec<u32>, Vec<u32>) {
        (self.front.borrow().clone(), self.back.borrow().clone())
    }
}

/// Packt für jede eingegangene Bestellung ein Paket; zwischen Beginn und Ende wartet es auf das Lager.
pub struct Packing {
    pub journal: Rc<RefCell<Vec<String>>>,
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

/// Schließt die Bestellung mit dieser Nummer ab.
pub struct Checkout {
    pub order_id: u32,
}

impl Request for Checkout {
    type Response = ();
    type Error = SendError;
}

/// Schließt Bestellungen ab und meldet sie über den Publisher, den er per Konstruktor bekommt.
pub struct CheckoutHandler<P> {
    pub publisher: P,
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

/// Attrappe statt des Mediators: merkt sich die Nummer jeder gemeldeten Bestellung.
pub struct RecordedOrders {
    pub order_ids: Rc<RefCell<Vec<u32>>>,
}

impl Publisher<OrderPlaced> for RecordedOrders {
    async fn publish(&self, event: OrderPlaced) -> Result<(), SendError> {
        self.order_ids.borrow_mut().push(event.order_id);
        Ok(())
    }
}

/// Ereignis: Die Bestellung mit dieser Nummer steht unter Betrugsverdacht.
pub struct FraudSuspected {
    pub order_id: u32,
}

impl Notification for FraudSuspected {
    type Error = SendError;
}

/// Prüft jede eingegangene Bestellung und meldet gesperrte Nummern über den Mediator, den sie per Konstruktor bekommt.
pub struct FraudCheck {
    pub publisher: Mediator,
    pub blocked: Vec<u32>,
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
pub struct FraudDesk {
    pub cases: Rc<RefCell<Vec<u32>>>,
}

impl NotificationHandler<FraudSuspected> for FraudDesk {
    async fn handle(&self, event: &FraudSuspected) -> Result<(), SendError> {
        self.cases.borrow_mut().push(event.order_id);
        Ok(())
    }
}

/// Sperrt den Kunden der Bestellung mit dieser Nummer.
pub struct BlockCustomer {
    pub order_id: u32,
}

impl Request for BlockCustomer {
    type Response = ();
    type Error = SendError;
}

/// Führt die Sperrliste und trägt jede Bestellnummer ein, deren Kunde gesperrt wird.
pub struct Blocklist {
    pub order_ids: Rc<RefCell<Vec<u32>>>,
}

impl RequestHandler<BlockCustomer> for Blocklist {
    async fn handle(&self, request: BlockCustomer) -> Result<(), SendError> {
        self.order_ids.borrow_mut().push(request.order_id);
        Ok(())
    }
}

/// Ermittelt bei jedem Betrugsverdacht und lässt den Kunden über den Mediator sperren, den sie per Konstruktor bekommt.
pub struct FraudInvestigator {
    pub sender: Mediator,
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
