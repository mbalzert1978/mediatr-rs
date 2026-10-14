//! Registriert ein Behavior nach dem ersten Handler; das darf nicht kompilieren.

use mediatr_rs::{Handler, Mediator, Next, PipelineBehavior, Request, RequestHandler, SendError};

/// Fragt den Lagerbestand des Artikels mit dieser Nummer ab.
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

/// Schreibt jeden Request ins Prüfprotokoll.
struct AuditTrail;

impl<R: Request> PipelineBehavior<R> for AuditTrail {
    async fn handle(&self, request: R, next: Next<R>) -> Result<R::Response, R::Error> {
        next.run(request).await
    }
}

fn main() {
    let _mediator = Mediator::builder()
        .register(Handler::instance(StockLedger))
        .open_behavior(AuditTrail)
        .build();
}
