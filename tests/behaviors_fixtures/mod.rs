use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use mediatr_rs::{
    Next, Notification, NotificationHandler, PipelineBehavior, Request, RequestHandler, SendError,
};

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

/// Belegt die Laderampe, solange ein Wareneingang gebucht wird.
pub struct LoadingDock {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl PipelineBehavior<BookReceipt> for LoadingDock {
    async fn handle(
        &self,
        request: BookReceipt,
        next: Next<BookReceipt>,
    ) -> Result<(), StockError> {
        self.journal.borrow_mut().push("Rampe belegt".to_owned());
        let booked = next.run(request).await;
        self.journal.borrow_mut().push("Rampe frei".to_owned());
        booked
    }
}

/// Prüfstation Nummer `N` am Lager; schreibt vor und nach jeder Bestandsabfrage ihre Nummer ins Protokoll.
pub struct Checkpoint<const N: usize> {
    pub journal: Rc<RefCell<Vec<String>>>,
}

/// Prüfstation Nummer `N`, die in `journal` schreibt.
pub fn checkpoint<const N: usize>(journal: &Rc<RefCell<Vec<String>>>) -> Checkpoint<N> {
    Checkpoint {
        journal: Rc::clone(journal),
    }
}

impl<const N: usize> PipelineBehavior<LookupStock> for Checkpoint<N> {
    async fn handle(
        &self,
        request: LookupStock,
        next: Next<LookupStock>,
    ) -> Result<u32, StockError> {
        self.journal
            .borrow_mut()
            .push(format!("Station {N} betreten"));
        let stock = next.run(request).await;
        self.journal
            .borrow_mut()
            .push(format!("Station {N} verlassen"));
        stock
    }
}

/// Was die Prüfstationen 0 bis `count - 1`, in dieser Reihenfolge registriert, um den Protokolleintrag `handled` herum ins Protokoll schreiben.
pub fn passed_checkpoints(count: usize, handled: &str) -> Vec<String> {
    let entered = (0..count).map(|n| format!("Station {n} betreten"));
    let left = (0..count).rev().map(|n| format!("Station {n} verlassen"));
    entered.chain([handled.to_owned()]).chain(left).collect()
}

/// Ereignis: Der Bestand des Artikels mit dieser Nummer wurde gezählt.
pub struct StockCounted {
    pub sku: u32,
}

impl Notification for StockCounted {
    type Error = SendError;
}

/// Druckt nach jeder Zählung ein neues Regaletikett; zwischen Beginn und Ende wartet es auf den Drucker.
pub struct ShelfLabeling {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<StockCounted> for ShelfLabeling {
    async fn handle(&self, event: &StockCounted) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Etikett beginnt {}", event.sku));
        tokio::task::yield_now().await;
        self.journal
            .borrow_mut()
            .push(format!("Etikett fertig {}", event.sku));
        Ok(())
    }
}

/// Prüft nach jeder Zählung, ob nachbestellt werden muss; zwischen Beginn und Ende wartet es auf den Lieferanten.
pub struct ReorderCheck {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<StockCounted> for ReorderCheck {
    async fn handle(&self, event: &StockCounted) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Nachbestellung beginnt {}", event.sku));
        tokio::task::yield_now().await;
        self.journal
            .borrow_mut()
            .push(format!("Nachbestellung fertig {}", event.sku));
        Ok(())
    }
}

/// Was [`ShelfLabeling`] und [`ReorderCheck`], gleichzeitig gestartet, zur Zählung des Artikels `sku` ins Protokoll schreiben.
pub fn labeled_and_reordered_together(sku: u32) -> Vec<String> {
    vec![
        format!("Etikett beginnt {sku}"),
        format!("Nachbestellung beginnt {sku}"),
        format!("Etikett fertig {sku}"),
        format!("Nachbestellung fertig {sku}"),
    ]
}

/// Kennt den Bestand aus dem Zwischenspeicher und fragt das Lager gar nicht erst.
pub struct StockCache {
    pub cached: u32,
}

impl PipelineBehavior<LookupStock> for StockCache {
    async fn handle(
        &self,
        _request: LookupStock,
        _next: Next<LookupStock>,
    ) -> Result<u32, StockError> {
        Ok(self.cached)
    }
}

/// Führt den Bestand im Außenlager; die Suche dort dauert eine Stunde.
pub struct RemoteWarehouse {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestHandler<LookupStock> for RemoteWarehouse {
    async fn handle(&self, request: LookupStock) -> Result<u32, StockError> {
        self.journal
            .borrow_mut()
            .push(format!("Außenlager sucht {}", request.sku));
        tokio::time::sleep(Duration::from_hours(1)).await;
        self.journal
            .borrow_mut()
            .push(format!("Außenlager fand {}", request.sku));
        Ok(12)
    }
}

/// Bricht eine Bestandsabfrage ab, die länger als `limit` dauert.
pub struct Deadline {
    pub limit: Duration,
}

impl PipelineBehavior<LookupStock> for Deadline {
    async fn handle(
        &self,
        request: LookupStock,
        next: Next<LookupStock>,
    ) -> Result<u32, StockError> {
        tokio::time::timeout(self.limit, next.run(request))
            .await
            .unwrap_or(Err(StockError::Timeout))
    }
}
