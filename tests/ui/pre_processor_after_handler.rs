//! Registriert einen Vorverarbeiter nach dem ersten Handler; das darf nicht kompilieren.

use mediatr_rs::{Handler, Mediator, Request, RequestHandler, RequestPreProcessor, SendError};

/// Fragt den Preis des Artikels mit dieser Nummer in Cent ab.
struct QuotePrice {
    sku: u32,
}

impl Request for QuotePrice {
    type Response = u32;
    type Error = SendError;
}

/// Berechnet Preise.
struct PriceList;

impl RequestHandler<QuotePrice> for PriceList {
    async fn handle(&self, request: QuotePrice) -> Result<u32, SendError> {
        Ok(request.sku)
    }
}

/// Stempelt jede eingehende Anfrage.
struct InboxStamp;

impl<R: Request> RequestPreProcessor<R> for InboxStamp {
    async fn process(&self, _request: &R) {}
}

fn main() {
    let _mediator = Mediator::builder()
        .register(Handler::instance(PriceList))
        .pre_processor(InboxStamp)
        .build();
}
