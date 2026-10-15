use mediatr_rs::{
    Next, PipelineBehavior, Request, RequestHandler, RequestPostProcessor, RequestPreProcessor,
    SendError,
};

use crate::common::Journal;

/// Fragt den Preis des Artikels mit dieser Nummer in Cent ab.
#[derive(Clone)]
pub struct QuotePrice {
    pub sku: u32,
}

/// Warum eine Preisanfrage scheitert.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PriceError {
    Discontinued,
    AboveCeiling,
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
    pub journal: Journal,
}

impl RequestHandler<QuotePrice> for PriceList {
    async fn handle(&self, request: QuotePrice) -> Result<u32, PriceError> {
        self.journal
            .push(format!("Preis berechnet für {}", request.sku));
        Ok(1999)
    }
}

/// Schreibt jede eingehende Preisanfrage ins Protokoll, bevor sie berechnet wird.
pub struct InquiryLog {
    pub journal: Journal,
}

impl RequestPreProcessor<QuotePrice> for InquiryLog {
    async fn process(&self, request: &QuotePrice) -> Result<(), PriceError> {
        self.journal
            .push(format!("Anfrage für {} eingegangen", request.sku));
        Ok(())
    }
}

/// Legt jeden berechneten Preis mit seiner Anfrage im Archiv ab.
pub struct QuoteArchive {
    pub journal: Journal,
}

impl RequestPostProcessor<QuotePrice> for QuoteArchive {
    async fn process(&self, request: &QuotePrice, response: &u32) -> Result<(), PriceError> {
        self.journal
            .push(format!("Preis {response} für {} archiviert", request.sku));
        Ok(())
    }
}

/// Prüfschritt Nummer `N` vor jeder Preisberechnung; schreibt in `journal`.
pub struct PriceCheck<const N: usize> {
    pub journal: Journal,
}

/// Prüfschritt Nummer `N`, der in `journal` schreibt.
pub fn price_check<const N: usize>(journal: &Journal) -> PriceCheck<N> {
    PriceCheck {
        journal: journal.clone(),
    }
}

impl<const N: usize> RequestPreProcessor<QuotePrice> for PriceCheck<N> {
    async fn process(&self, _request: &QuotePrice) -> Result<(), PriceError> {
        self.journal.push(format!("Prüfung {N}"));
        Ok(())
    }
}

/// Export Nummer `N` nach jeder Preisberechnung; schreibt in `journal`.
pub struct PriceExport<const N: usize> {
    pub journal: Journal,
}

/// Export Nummer `N`, der in `journal` schreibt.
pub fn price_export<const N: usize>(journal: &Journal) -> PriceExport<N> {
    PriceExport {
        journal: journal.clone(),
    }
}

impl<const N: usize> RequestPostProcessor<QuotePrice> for PriceExport<N> {
    async fn process(&self, _request: &QuotePrice, _response: &u32) -> Result<(), PriceError> {
        self.journal.push(format!("Export {N}"));
        Ok(())
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
    pub journal: Journal,
}

impl RequestHandler<QuoteShipping> for ShippingTable {
    async fn handle(&self, request: QuoteShipping) -> Result<u32, SendError> {
        self.journal
            .push(format!("Versand berechnet für {}", request.sku));
        Ok(490)
    }
}

/// Stempelt jede eingehende Anfrage im Protokoll.
pub struct InboxStamp {
    pub journal: Journal,
}

/// Eingangsstempel, der in `journal` schreibt.
pub fn inbox_stamp(journal: &Journal) -> InboxStamp {
    InboxStamp {
        journal: journal.clone(),
    }
}

impl<R: Request> RequestPreProcessor<R> for InboxStamp {
    async fn process(&self, _request: &R) -> Result<(), R::Error> {
        self.journal.push("Eingang gestempelt");
        Ok(())
    }
}

/// Stempelt jede beantwortete Anfrage im Protokoll.
pub struct OutboxStamp {
    pub journal: Journal,
}

/// Ausgangsstempel, der in `journal` schreibt.
pub fn outbox_stamp(journal: &Journal) -> OutboxStamp {
    OutboxStamp {
        journal: journal.clone(),
    }
}

impl<R: Request> RequestPostProcessor<R> for OutboxStamp {
    async fn process(&self, _request: &R, _response: &R::Response) -> Result<(), R::Error> {
        self.journal.push("Ausgang gestempelt");
        Ok(())
    }
}

/// Misst jede Anfrage und schreibt Start und Ende ins Protokoll.
pub struct Timer {
    pub journal: Journal,
}

/// Zeitmessung, die in `journal` schreibt.
pub fn timer(journal: &Journal) -> Timer {
    Timer {
        journal: journal.clone(),
    }
}

impl<R: Request> PipelineBehavior<R> for Timer {
    async fn handle(&self, request: R, next: Next<R>) -> Result<R::Response, R::Error> {
        self.journal.push("Zeit startet");
        let response = next.run(request).await;
        self.journal.push("Zeit stoppt");
        response
    }
}

/// Sperrt den Preis, solange er berechnet wird.
pub struct PriceLock {
    pub journal: Journal,
}

/// Preissperre, die in `journal` schreibt.
pub fn price_lock(journal: &Journal) -> PriceLock {
    PriceLock {
        journal: journal.clone(),
    }
}

impl PipelineBehavior<QuotePrice> for PriceLock {
    async fn handle(&self, request: QuotePrice, next: Next<QuotePrice>) -> Result<u32, PriceError> {
        self.journal.push("Preis gesperrt");
        let price = next.run(request).await;
        self.journal.push("Preis freigegeben");
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
    pub journal: Journal,
}

impl RequestHandler<QuotePrice> for RetiredItems {
    async fn handle(&self, request: QuotePrice) -> Result<u32, PriceError> {
        self.journal
            .push(format!("Artikel {} ausgelaufen", request.sku));
        Err(PriceError::Discontinued)
    }
}

/// Lässt keine Preisanfrage für den ausgelaufenen Artikel `retired` durch und schreibt jede Ablehnung ins Protokoll.
pub struct RetiredFilter {
    pub retired: u32,
    pub journal: Journal,
}

impl RequestPreProcessor<QuotePrice> for RetiredFilter {
    async fn process(&self, request: &QuotePrice) -> Result<(), PriceError> {
        if request.sku != self.retired {
            return Ok(());
        }
        self.journal
            .push(format!("Anfrage für {} abgelehnt", request.sku));
        Err(PriceError::Discontinued)
    }
}

/// Lässt keinen berechneten Preis über `max` hinaus und schreibt jede Überschreitung ins Protokoll.
pub struct PriceCeiling {
    pub max: u32,
    pub journal: Journal,
}

impl RequestPostProcessor<QuotePrice> for PriceCeiling {
    async fn process(&self, _request: &QuotePrice, response: &u32) -> Result<(), PriceError> {
        if *response <= self.max {
            return Ok(());
        }
        self.journal
            .push(format!("Preis {response} über Obergrenze {}", self.max));
        Err(PriceError::AboveCeiling)
    }
}
