//! Registriert einen Nachverarbeiter nach dem ersten Handler; das darf nicht kompilieren.

use mediatr_rs::{Handler, Mediator, Request, RequestHandler, RequestPostProcessor, SendError};

/// Fragt die Versandkosten des Artikels mit dieser Nummer in Cent ab.
#[derive(Clone)]
struct QuoteShipping {
    sku: u32,
}

impl Request for QuoteShipping {
    type Response = u32;
    type Error = SendError;
}

/// Berechnet Versandkosten.
struct ShippingTable;

impl RequestHandler<QuoteShipping> for ShippingTable {
    async fn handle(&self, request: QuoteShipping) -> Result<u32, SendError> {
        Ok(request.sku)
    }
}

/// Stempelt jede beantwortete Anfrage.
struct OutboxStamp;

impl<R: Request> RequestPostProcessor<R> for OutboxStamp {
    async fn process(&self, _request: &R, _response: &R::Response) -> Result<(), R::Error> {
        Ok(())
    }
}

fn main() {
    let _mediator = Mediator::builder()
        .register(Handler::instance(ShippingTable))
        .post_processor(OutboxStamp)
        .build();
}
