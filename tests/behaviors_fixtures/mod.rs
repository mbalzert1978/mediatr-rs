use std::cell::{Cell, RefCell};
use std::rc::Rc;

use mediatr_rs::{Next, PipelineBehavior, Request, RequestHandler, SendError};

/// Fragt den Lagerbestand des Artikels mit dieser Nummer ab.
pub struct LookupStock {
    pub sku: u32,
}

/// Warum eine Bestandsabfrage scheitert.
#[derive(Debug, PartialEq)]
pub enum StockError {
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
pub struct SlowStockLedger {
    pub calls: Rc<Cell<u32>>,
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
pub struct RetryOnce;

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

/// Führt den Lagerbestand und schreibt jede Abfrage ins Protokoll.
pub struct StockLedger {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestHandler<LookupStock> for StockLedger {
    async fn handle(&self, request: LookupStock) -> Result<u32, StockError> {
        self.journal
            .borrow_mut()
            .push(format!("Bestand gelesen für {}", request.sku));
        Ok(12)
    }
}

/// Schreibt vor und nach jedem Request einen Eintrag ins Prüfprotokoll.
pub struct AuditTrail {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl<R: Request> PipelineBehavior<R> for AuditTrail {
    async fn handle(&self, request: R, next: Next<R>) -> Result<R::Response, R::Error> {
        self.journal.borrow_mut().push("Prüfung beginnt".to_owned());
        let response = next.run(request).await;
        self.journal.borrow_mut().push("Prüfung endet".to_owned());
        response
    }
}

/// Misst jeden Request und schreibt Start und Ende ins Protokoll.
pub struct Stopwatch {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl<R: Request> PipelineBehavior<R> for Stopwatch {
    async fn handle(&self, request: R, next: Next<R>) -> Result<R::Response, R::Error> {
        self.journal.borrow_mut().push("Messung startet".to_owned());
        let response = next.run(request).await;
        self.journal.borrow_mut().push("Messung stoppt".to_owned());
        response
    }
}

/// Prüfprotokoll und Stoppuhr, die beide in `journal` schreiben; die Prüfung liegt außen.
pub fn audit_and_stopwatch(journal: &Rc<RefCell<Vec<String>>>) -> (AuditTrail, Stopwatch) {
    (
        AuditTrail {
            journal: Rc::clone(journal),
        },
        Stopwatch {
            journal: Rc::clone(journal),
        },
    )
}

/// Was [`audit_and_stopwatch`] um den Protokolleintrag `handled` eines Handlers herum ins Protokoll schreibt.
pub fn audited_and_timed(handled: &str) -> Vec<String> {
    [
        "Prüfung beginnt",
        "Messung startet",
        handled,
        "Messung stoppt",
        "Prüfung endet",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// Bucht den Wareneingang des Artikels mit dieser Nummer; die Buchung hat keine Antwort.
pub struct BookReceipt {
    pub sku: u32,
}

impl Request for BookReceipt {
    type Response = ();
    type Error = StockError;
}

/// Bucht jeden Wareneingang und schreibt das ins Protokoll.
pub struct ReceiptBook {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestHandler<BookReceipt> for ReceiptBook {
    async fn handle(&self, request: BookReceipt) -> Result<(), StockError> {
        self.journal
            .borrow_mut()
            .push(format!("Eingang gebucht für {}", request.sku));
        Ok(())
    }
}

/// Sperrt das Regal, solange der Bestand eines Artikels gelesen wird.
pub struct ShelfLock {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl PipelineBehavior<LookupStock> for ShelfLock {
    async fn handle(
        &self,
        request: LookupStock,
        next: Next<LookupStock>,
    ) -> Result<u32, StockError> {
        self.journal.borrow_mut().push("Regal gesperrt".to_owned());
        let stock = next.run(request).await;
        self.journal.borrow_mut().push("Regal frei".to_owned());
        stock
    }
}

/// Was Prüfprotokoll, [`ShelfLock`] und Stoppuhr, in dieser Reihenfolge registriert, um den Protokolleintrag `handled` herum ins Protokoll schreiben.
pub fn audited_locked_and_timed(handled: &str) -> Vec<String> {
    [
        "Prüfung beginnt",
        "Regal gesperrt",
        "Messung startet",
        handled,
        "Messung stoppt",
        "Regal frei",
        "Prüfung endet",
    ]
    .map(str::to_owned)
    .to_vec()
}
