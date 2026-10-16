//! Die Fehler-Aktion, die auf einen Fehler der Pipeline reagiert.

use std::future::Future;

use crate::pipeline::RequestErrorActionProcessorBehavior;
use crate::request::HandlerResult;
use crate::{Next, PipelineBehavior, Request};

/// Reagiert auf den Fehler eines Requests vom Typ `R`; der Fehler geht danach weiter, außer die Aktion scheitert selbst.
///
/// # Vertrag
///
/// `execute` endet für jeden Fehler mit `Ok` oder einem eigenen Fehler und löst keine Panic aus.
/// Die Fehler-Aktion ist `Send + Sync` und ihre Future `Send`: Mehrere Threads teilen sich eine Fehler-Aktion.
pub trait RequestErrorAction<R: Request>: Send + Sync {
    /// Sieht `request` mit dem `error`, den die innere Pipeline (Behaviors und Handler) geliefert hat.
    ///
    /// # Errors
    ///
    /// `R::Error`, wenn die Aktion selbst scheitert; dann liefert `send` diesen Fehler statt `error`.
    fn execute(
        &self,
        request: &R,
        error: &R::Error,
    ) -> impl Future<Output = Result<(), R::Error>> + Send;
}

impl<R: Request + Clone, B: RequestErrorAction<R>> PipelineBehavior<R>
    for RequestErrorActionProcessorBehavior<B>
{
    async fn handle(&self, request: R, next: Next<R>) -> HandlerResult<R> {
        let copy = request.clone();
        let response = next.run(request).await;
        if let Err(error) = &response {
            self.0.execute(&copy, error).await?;
        }
        response
    }
}
