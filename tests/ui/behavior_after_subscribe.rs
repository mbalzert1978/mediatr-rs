//! Registriert ein Behavior nach dem ersten Notification-Handler; das darf nicht kompilieren.

use mediatr_rs::{
    Handler, Mediator, Next, Notification, NotificationHandler, PipelineBehavior, Request,
    SendError,
};

/// Ereignis: Der Bestand des Artikels mit dieser Nummer wurde gezählt.
struct StockCounted {
    sku: u32,
}

impl Notification for StockCounted {
    type Error = SendError;
}

/// Druckt nach jeder Zählung ein neues Regaletikett.
struct ShelfLabeling;

impl NotificationHandler<StockCounted> for ShelfLabeling {
    async fn handle(&self, event: &StockCounted) -> Result<(), SendError> {
        let _ = event.sku;
        Ok(())
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
        .subscribe(Handler::instance(ShelfLabeling))
        .open_behavior(AuditTrail)
        .build();
}
