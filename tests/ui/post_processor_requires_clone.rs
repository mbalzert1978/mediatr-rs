//! Ein Nachverarbeiter ist registriert, der Request-Typ lässt sich nicht klonen; `send` darf nicht kompilieren.

use mediatr_rs::{
    Handler, Mediator, Request, RequestHandler, RequestPostProcessor, SendError, Sender,
};

/// Fragt den Preis des Artikels mit dieser Nummer in Cent ab; lässt sich nicht klonen.
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

/// Legt jede Antwort im Archiv ab; braucht selbst kein `Clone`.
struct ReplyArchive;

impl<R: Request> RequestPostProcessor<R> for ReplyArchive {
    async fn process(&self, _request: &R, _response: &R::Response) -> Result<(), R::Error> {
        Ok(())
    }
}

fn main() {
    let mediator = Mediator::builder()
        .post_processor(ReplyArchive)
        .register(Handler::instance(PriceList))
        .build();
    let _price = mediator.send(QuotePrice { sku: 7 });
}
