//! Tests für den Abschnitt „Pipeline — Behaviors“ in `TESTLIST.md`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use mediatr_rs::{
    Handler, Mediator, Next, PipelineBehavior, Request, RequestHandler, SendError, Sender,
};

/// Fragt den Lagerbestand des Artikels mit dieser Nummer ab.
struct LookupStock {
    sku: u32,
}

/// Warum eine Bestandsabfrage scheitert.
#[derive(Debug, PartialEq)]
enum StockError {
    Timeout,
    Undeliverable(SendError),
}

impl From<SendError> for StockError {
    fn from(error: SendError) -> Self {
        Self::Undeliverable(error)
    }
}

impl Request for LookupStock {
    type Response = u32;
    type Error = StockError;
}

/// Führt den Lagerbestand; der Lagerserver antwortet auf die erste Abfrage zu spät.
struct SlowStockLedger {
    calls: Rc<Cell<u32>>,
}

impl RequestHandler<LookupStock> for SlowStockLedger {
    async fn handle(&self, _request: LookupStock) -> Result<u32, StockError> {
        let call = self.calls.get().saturating_add(1);
        self.calls.set(call);
        if call == 1 {
            return Err(StockError::Timeout);
        }
        Ok(12)
    }
}

/// Wiederholt eine Bestandsabfrage einmal, wenn sie scheitert.
struct RetryOnce;

impl PipelineBehavior<LookupStock> for RetryOnce {
    async fn handle(
        &self,
        request: LookupStock,
        next: Next<LookupStock>,
    ) -> Result<u32, StockError> {
        let sku = request.sku;
        match next.clone().run(request).await {
            Ok(stock) => Ok(stock),
            Err(_) => next.run(LookupStock { sku }).await,
        }
    }
}

#[tokio::test]
async fn behavior_retries_cloned_next_after_handler_error_and_returns_second_response() {
    let calls = Rc::new(Cell::new(0));
    let mediator = Mediator::builder()
        .behavior(Handler::instance(RetryOnce))
        .register(Handler::instance(SlowStockLedger {
            calls: Rc::clone(&calls),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!((stock, calls.get()), (Ok(12), 2));
}

/// Führt den Lagerbestand und schreibt jede Abfrage ins Protokoll.
struct StockLedger {
    journal: Rc<RefCell<Vec<String>>>,
}

impl RequestHandler<LookupStock> for StockLedger {
    async fn handle(&self, request: LookupStock) -> Result<u32, StockError> {
        self.journal
            .borrow_mut()
            .push(format!("Bestand gelesen für {}", request.sku));
        Ok(12)
    }
}

/// Schreibt vor und nach jeder Bestandsabfrage einen Eintrag ins Prüfprotokoll.
struct AuditTrail {
    journal: Rc<RefCell<Vec<String>>>,
}

impl PipelineBehavior<LookupStock> for AuditTrail {
    async fn handle(
        &self,
        request: LookupStock,
        next: Next<LookupStock>,
    ) -> Result<u32, StockError> {
        self.journal.borrow_mut().push("Prüfung beginnt".to_owned());
        let stock = next.run(request).await;
        self.journal.borrow_mut().push("Prüfung endet".to_owned());
        stock
    }
}

/// Misst jede Bestandsabfrage und schreibt Start und Ende ins Protokoll.
struct Stopwatch {
    journal: Rc<RefCell<Vec<String>>>,
}

impl PipelineBehavior<LookupStock> for Stopwatch {
    async fn handle(
        &self,
        request: LookupStock,
        next: Next<LookupStock>,
    ) -> Result<u32, StockError> {
        self.journal.borrow_mut().push("Messung startet".to_owned());
        let stock = next.run(request).await;
        self.journal.borrow_mut().push("Messung stoppt".to_owned());
        stock
    }
}

#[tokio::test]
async fn two_behaviors_wrap_handler_first_registered_outermost() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .behavior(Handler::instance(AuditTrail {
            journal: Rc::clone(&journal),
        }))
        .behavior(Handler::instance(Stopwatch {
            journal: Rc::clone(&journal),
        }))
        .register(Handler::instance(StockLedger {
            journal: Rc::clone(&journal),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.borrow().clone()),
        (
            Ok(12),
            vec![
                "Prüfung beginnt".to_owned(),
                "Messung startet".to_owned(),
                "Bestand gelesen für 4".to_owned(),
                "Messung stoppt".to_owned(),
                "Prüfung endet".to_owned()
            ]
        )
    );
}

/// Bucht den Wareneingang des Artikels mit dieser Nummer; die Buchung hat keine Antwort.
struct BookReceipt {
    sku: u32,
}

impl Request for BookReceipt {
    type Response = ();
    type Error = StockError;
}

/// Bucht jeden Wareneingang und schreibt das ins Protokoll.
struct ReceiptBook {
    journal: Rc<RefCell<Vec<String>>>,
}

impl RequestHandler<BookReceipt> for ReceiptBook {
    async fn handle(&self, request: BookReceipt) -> Result<(), StockError> {
        self.journal
            .borrow_mut()
            .push(format!("Eingang gebucht für {}", request.sku));
        Ok(())
    }
}

/// Schreibt vor und nach jeder Eingangsbuchung einen Eintrag ins Prüfprotokoll.
struct ReceiptAudit {
    journal: Rc<RefCell<Vec<String>>>,
}

impl PipelineBehavior<BookReceipt> for ReceiptAudit {
    async fn handle(
        &self,
        request: BookReceipt,
        next: Next<BookReceipt>,
    ) -> Result<(), StockError> {
        self.journal.borrow_mut().push("Prüfung beginnt".to_owned());
        let booked = next.run(request).await;
        self.journal.borrow_mut().push("Prüfung endet".to_owned());
        booked
    }
}

/// Misst jede Eingangsbuchung und schreibt Start und Ende ins Protokoll.
struct ReceiptStopwatch {
    journal: Rc<RefCell<Vec<String>>>,
}

impl PipelineBehavior<BookReceipt> for ReceiptStopwatch {
    async fn handle(
        &self,
        request: BookReceipt,
        next: Next<BookReceipt>,
    ) -> Result<(), StockError> {
        self.journal.borrow_mut().push("Messung startet".to_owned());
        let booked = next.run(request).await;
        self.journal.borrow_mut().push("Messung stoppt".to_owned());
        booked
    }
}

#[tokio::test]
async fn two_behaviors_wrap_unit_response_handler_first_registered_outermost() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .behavior(Handler::instance(ReceiptAudit {
            journal: Rc::clone(&journal),
        }))
        .behavior(Handler::instance(ReceiptStopwatch {
            journal: Rc::clone(&journal),
        }))
        .register(Handler::instance(ReceiptBook {
            journal: Rc::clone(&journal),
        }))
        .build();

    let booked = mediator.send(BookReceipt { sku: 4 }).await;

    assert_eq!(
        (booked, journal.borrow().clone()),
        (
            Ok(()),
            vec![
                "Prüfung beginnt".to_owned(),
                "Messung startet".to_owned(),
                "Eingang gebucht für 4".to_owned(),
                "Messung stoppt".to_owned(),
                "Prüfung endet".to_owned()
            ]
        )
    );
}
