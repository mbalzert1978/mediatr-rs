//! Der Fehler-Handler, der für einen Fehler der Pipeline eine Ersatzantwort liefern kann.

use std::future::Future;

use crate::pipeline::RequestErrorProcessorBehavior;
use crate::request::HandlerResult;
use crate::{Next, PipelineBehavior, Request};

/// Behandelt den Fehler eines Requests vom Typ `R`: liefert eine Ersatzantwort oder reicht einen Fehler weiter.
///
/// # Vertrag
///
/// `handle` liefert für jeden Fehler genau eine Ersatzantwort oder einen Fehler und löst keine Panic aus.
pub trait RequestErrorHandler<R: Request> {
    /// Bekommt `request` mit dem `error`, den die innere Pipeline (Behaviors und Handler) geliefert hat.
    ///
    /// # Errors
    ///
    /// `R::Error`, wenn der Handler keine Ersatzantwort hat: `error` selbst, um ihn weiterzureichen, oder einen eigenen Fehler.
    fn handle(&self, request: &R, error: R::Error) -> impl Future<Output = HandlerResult<R>>;
}

impl<R: Request + Clone, B: RequestErrorHandler<R>> PipelineBehavior<R>
    for RequestErrorProcessorBehavior<B>
{
    async fn handle(&self, request: R, next: Next<R>) -> HandlerResult<R> {
        let copy = request.clone();
        match next.run(request).await {
            Err(error) => self.0.handle(&copy, error).await,
            response => response,
        }
    }
}
