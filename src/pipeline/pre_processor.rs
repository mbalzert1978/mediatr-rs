//! Der Vorverarbeiter, der vor dem Handler den Request sieht.

use std::future::Future;

use crate::pipeline::RequestPreProcessorBehavior;
use crate::request::HandlerResult;
use crate::{Next, PipelineBehavior, Request};

/// Läuft vor dem Handler von Requests vom Typ `R` und sieht den Request.
///
/// # Vertrag
///
/// `process` endet für jeden Request mit `Ok` oder einem Fehler und löst keine Panic aus.
/// Der Vorverarbeiter ist `Send + Sync` und seine Future `Send`: Mehrere Threads teilen sich einen Vorverarbeiter.
pub trait RequestPreProcessor<R: Request>: Send + Sync {
    /// Sieht `request`, bevor der Handler ihn bekommt.
    ///
    /// # Errors
    ///
    /// `R::Error`, wenn `request` nicht weiter darf; dann läuft der Handler nicht, und `send` liefert diesen Fehler.
    fn process(&self, request: &R) -> impl Future<Output = Result<(), R::Error>> + Send;
}

impl<R: Request, B: RequestPreProcessor<R>> PipelineBehavior<R> for RequestPreProcessorBehavior<B> {
    async fn handle(&self, request: R, next: Next<R>) -> HandlerResult<R> {
        self.0.process(&request).await?;
        next.run(request).await
    }
}
