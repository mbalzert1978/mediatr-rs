//! Der Nachverarbeiter, der nach dem Handler Request samt Antwort sieht.

use std::future::Future;

use crate::pipeline::RequestPostProcessorBehavior;
use crate::request::HandlerResult;
use crate::{Next, PipelineBehavior, Request};

/// Läuft nach dem Handler von Requests vom Typ `R` und sieht Request samt Antwort.
///
/// # Vertrag
///
/// `process` endet für jeden Request mit `Ok` oder einem Fehler und löst keine Panic aus.
/// Der Nachverarbeiter ist `Send + Sync` und seine Future `Send`: Mehrere Threads teilen sich einen Nachverarbeiter.
pub trait RequestPostProcessor<R: Request>: Send + Sync {
    /// Sieht `request` mit der `response` der inneren Pipeline (Behaviors und Handler).
    ///
    /// # Errors
    ///
    /// `R::Error`, wenn `response` nicht hinaus darf; dann liefert `send` diesen Fehler statt der Antwort.
    fn process(
        &self,
        request: &R,
        response: &R::Response,
    ) -> impl Future<Output = Result<(), R::Error>> + Send;
}

impl<R: Request + Clone, B: RequestPostProcessor<R>> PipelineBehavior<R>
    for RequestPostProcessorBehavior<B>
{
    async fn handle(&self, request: R, next: Next<R>) -> HandlerResult<R> {
        let copy = request.clone();
        let response = next.run(request).await?;
        self.0.process(&copy, &response).await?;
        Ok(response)
    }
}
