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
