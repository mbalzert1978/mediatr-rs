use std::cell::RefCell;
use std::rc::Rc;

use mediatr_rs::{Request, RequestHandler, RequestPostProcessor, RequestPreProcessor, SendError};

/// Fragt den Preis des Artikels mit dieser Nummer in Cent ab.
#[derive(Clone)]
pub struct QuotePrice {
    pub sku: u32,
}

impl Request for QuotePrice {
    type Response = u32;
    type Error = SendError;
}

/// Berechnet Preise und schreibt jede Berechnung ins Protokoll.
pub struct PriceList {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestHandler<QuotePrice> for PriceList {
    async fn handle(&self, request: QuotePrice) -> Result<u32, SendError> {
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
