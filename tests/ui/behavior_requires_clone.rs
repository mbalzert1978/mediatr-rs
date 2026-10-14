//! Ein Behavior verlangt `Clone` vom Request, der Request-Typ erfüllt das nicht; `send` darf nicht kompilieren.

use mediatr_rs::{
    Handler, Mediator, Next, PipelineBehavior, Request, RequestHandler, SendError, Sender,
};

/// Fragt den Lagerbestand des Artikels mit dieser Nummer ab; lässt sich nicht klonen.
struct LookupStock {
    sku: u32,
}

impl Request for LookupStock {
    type Response = u32;
    type Error = SendError;
}

/// Führt den Lagerbestand.
struct StockLedger;

impl RequestHandler<LookupStock> for StockLedger {
    async fn handle(&self, request: LookupStock) -> Result<u32, SendError> {
        Ok(request.sku)
    }
}

/// Schickt jeden Request nach einem Fehler mit einer Kopie noch einmal los; braucht dafür `Clone`.
struct RetryWithCopy;

impl<R: Request + Clone> PipelineBehavior<R> for RetryWithCopy {
    async fn handle(&self, request: R, next: Next<R>) -> Result<R::Response, R::Error> {
        match next.clone().run(request.clone()).await {
            Ok(response) => Ok(response),
            Err(_) => next.run(request).await,
        }
    }
}

fn main() {
    let mediator = Mediator::builder()
        .behavior(RetryWithCopy)
        .register(Handler::instance(StockLedger))
        .build();
    let _stock = mediator.send(LookupStock { sku: 4 });
}
