use std::cell::RefCell;
use std::rc::Rc;

use mediatr_rs::{
    Next, PipelineBehavior, Request, RequestHandler, RequestPostProcessor, RequestPreProcessor,
    SendError,
};

/// Fragt den Preis des Artikels mit dieser Nummer in Cent ab.
#[derive(Clone)]
pub struct QuotePrice {
    pub sku: u32,
}

/// Warum eine Preisanfrage scheitert.
#[derive(Debug, PartialEq)]
pub enum PriceError {
    Discontinued,
    Undeliverable(SendError),
}

impl From<SendError> for PriceError {
    fn from(error: SendError) -> Self {
        Self::Undeliverable(error)
    }
}

impl Request for QuotePrice {
    type Response = u32;
    type Error = PriceError;
}

/// Berechnet Preise und schreibt jede Berechnung ins Protokoll.
pub struct PriceList {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestHandler<QuotePrice> for PriceList {
    async fn handle(&self, request: QuotePrice) -> Result<u32, PriceError> {
        self.journal
            .borrow_mut()
            .push(format!("Preis berechnet für {}", request.sku));
        Ok(1999)
    }
}

/// Schreibt jede eingehende Preisanfrage ins Protokoll, bevor sie berechnet wird.
pub struct InquiryLog {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestPreProcessor<QuotePrice> for InquiryLog {
    async fn process(&self, request: &QuotePrice) {
        self.journal
            .borrow_mut()
            .push(format!("Anfrage für {} eingegangen", request.sku));
    }
}

/// Legt jeden berechneten Preis mit seiner Anfrage im Archiv ab.
pub struct QuoteArchive {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestPostProcessor<QuotePrice> for QuoteArchive {
    async fn process(&self, request: &QuotePrice, response: &u32) {
        self.journal
            .borrow_mut()
            .push(format!("Preis {response} für {} archiviert", request.sku));
    }
}

/// Prüfschritt Nummer `N` vor jeder Preisberechnung; schreibt in `journal`.
pub struct PriceCheck<const N: usize> {
    pub journal: Rc<RefCell<Vec<String>>>,
}

/// Prüfschritt Nummer `N`, der in `journal` schreibt.
pub fn price_check<const N: usize>(journal: &Rc<RefCell<Vec<String>>>) -> PriceCheck<N> {
    PriceCheck {
        journal: Rc::clone(journal),
    }
}

impl<const N: usize> RequestPreProcessor<QuotePrice> for PriceCheck<N> {
    async fn process(&self, _request: &QuotePrice) {
        self.journal.borrow_mut().push(format!("Prüfung {N}"));
    }
}

/// Export Nummer `N` nach jeder Preisberechnung; schreibt in `journal`.
pub struct PriceExport<const N: usize> {
    pub journal: Rc<RefCell<Vec<String>>>,
}

/// Export Nummer `N`, der in `journal` schreibt.
pub fn price_export<const N: usize>(journal: &Rc<RefCell<Vec<String>>>) -> PriceExport<N> {
    PriceExport {
        journal: Rc::clone(journal),
    }
}

impl<const N: usize> RequestPostProcessor<QuotePrice> for PriceExport<N> {
    async fn process(&self, _request: &QuotePrice, _response: &u32) {
        self.journal.borrow_mut().push(format!("Export {N}"));
    }
}

/// Was `count` Prüfschritte und `count` Exporte ab Nummer 1 um den Protokolleintrag `handled` eines Handlers herum schreiben.
pub fn checked_and_exported(count: usize, handled: &str) -> Vec<String> {
    let checks = (1..=count).map(|n| format!("Prüfung {n}"));
    let exports = (1..=count).map(|n| format!("Export {n}"));
    checks.chain([handled.to_owned()]).chain(exports).collect()
}

/// Fragt die Versandkosten des Artikels mit dieser Nummer in Cent ab.
#[derive(Clone)]
pub struct QuoteShipping {
    pub sku: u32,
}

impl Request for QuoteShipping {
    type Response = u32;
    type Error = SendError;
}

/// Berechnet Versandkosten und schreibt jede Berechnung ins Protokoll.
pub struct ShippingTable {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestHandler<QuoteShipping> for ShippingTable {
    async fn handle(&self, request: QuoteShipping) -> Result<u32, SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Versand berechnet für {}", request.sku));
        Ok(490)
    }
}

/// Stempelt jede eingehende Anfrage im Protokoll.
pub struct InboxStamp {
    pub journal: Rc<RefCell<Vec<String>>>,
}

/// Eingangsstempel, der in `journal` schreibt.
pub fn inbox_stamp(journal: &Rc<RefCell<Vec<String>>>) -> InboxStamp {
    InboxStamp {
        journal: Rc::clone(journal),
    }
}

impl<R: Request> RequestPreProcessor<R> for InboxStamp {
    async fn process(&self, _request: &R) {
        self.journal
            .borrow_mut()
            .push("Eingang gestempelt".to_owned());
    }
}

/// Stempelt jede beantwortete Anfrage im Protokoll.
pub struct OutboxStamp {
    pub journal: Rc<RefCell<Vec<String>>>,
}

/// Ausgangsstempel, der in `journal` schreibt.
pub fn outbox_stamp(journal: &Rc<RefCell<Vec<String>>>) -> OutboxStamp {
    OutboxStamp {
        journal: Rc::clone(journal),
    }
}

impl<R: Request> RequestPostProcessor<R> for OutboxStamp {
    async fn process(&self, _request: &R, _response: &R::Response) {
        self.journal
            .borrow_mut()
            .push("Ausgang gestempelt".to_owned());
    }
}

/// Misst jede Anfrage und schreibt Start und Ende ins Protokoll.
pub struct Timer {
    pub journal: Rc<RefCell<Vec<String>>>,
}

/// Zeitmessung, die in `journal` schreibt.
pub fn timer(journal: &Rc<RefCell<Vec<String>>>) -> Timer {
    Timer {
        journal: Rc::clone(journal),
    }
}

impl<R: Request> PipelineBehavior<R> for Timer {
    async fn handle(&self, request: R, next: Next<R>) -> Result<R::Response, R::Error> {
        self.journal.borrow_mut().push("Zeit startet".to_owned());
        let response = next.run(request).await;
        self.journal.borrow_mut().push("Zeit stoppt".to_owned());
        response
    }
}

/// Sperrt den Preis, solange er berechnet wird.
pub struct PriceLock {
    pub journal: Rc<RefCell<Vec<String>>>,
}

/// Preissperre, die in `journal` schreibt.
pub fn price_lock(journal: &Rc<RefCell<Vec<String>>>) -> PriceLock {
    PriceLock {
        journal: Rc::clone(journal),
    }
}

impl PipelineBehavior<QuotePrice> for PriceLock {
    async fn handle(&self, request: QuotePrice, next: Next<QuotePrice>) -> Result<u32, PriceError> {
        self.journal.borrow_mut().push("Preis gesperrt".to_owned());
        let price = next.run(request).await;
        self.journal
            .borrow_mut()
            .push("Preis freigegeben".to_owned());
        price
    }
}

/// Was ein Preis für Artikel 7 durch alle Stufen ins Protokoll schreibt.
pub fn price_through_all_stages() -> Vec<String> {
    [
        "Prüfung 1",
        "Eingang gestempelt",
        "Zeit startet",
        "Preis gesperrt",
        "Preis berechnet für 7",
        "Preis freigegeben",
        "Zeit stoppt",
        "Ausgang gestempelt",
        "Export 1",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// Was Versandkosten für Artikel 7 nur durch die allgemeinen Stufen ins Protokoll schreiben.
pub fn shipping_through_general_stages() -> Vec<String> {
    [
        "Eingang gestempelt",
        "Zeit startet",
        "Versand berechnet für 7",
        "Zeit stoppt",
        "Ausgang gestempelt",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// Preisliste für ausgelaufene Artikel: Sie schreibt die Anfrage ins Protokoll und kennt keinen Preis mehr.
pub struct RetiredItems {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestHandler<QuotePrice> for RetiredItems {
    async fn handle(&self, request: QuotePrice) -> Result<u32, PriceError> {
        self.journal
            .borrow_mut()
            .push(format!("Artikel {} ausgelaufen", request.sku));
        Err(PriceError::Discontinued)
    }
}
