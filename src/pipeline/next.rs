//! Der Rest der Pipeline hinter einem Behavior.

use std::fmt;
use std::sync::Arc;

use crate::Request;
use crate::request::HandlerResult;
use crate::wrappers::{BehaviorWrapper, RequestHandlerWrapper};

/// Der Rest der Pipeline hinter einem Behavior; [`Next::run`] schickt einen Request hinein.
pub struct Next<R: Request> {
    handler: Arc<RequestHandlerWrapper<R>>,
    /// Die noch ausstehenden Behaviors, das innerste zuerst; `run` nimmt das letzte, also äußerste.
    behaviors: Vec<Arc<BehaviorWrapper<R>>>,
}

impl<R: Request> Next<R> {
    /// Pipeline aus `behaviors`, geordnet wie [`order::behaviors`](crate::pipeline::order::behaviors) sie liefert, und `handler`.
    pub(crate) fn new(
        handler: Arc<RequestHandlerWrapper<R>>,
        behaviors: Vec<Arc<BehaviorWrapper<R>>>,
    ) -> Self {
        Self { handler, behaviors }
    }

    /// Schickt `request` an den nächsten Schritt und liefert dessen Antwort.
    ///
    /// # Errors
    ///
    /// Den Fehler des nächsten Schritts.
    pub async fn run(mut self, request: R) -> HandlerResult<R> {
        match self.behaviors.pop() {
            Some(behavior) => behavior(request, self).await,
            None => (self.handler)(request).await,
        }
    }
}

// Von Hand statt `derive`: `derive(Clone)` verlangte `R: Clone`, obwohl nur die `Arc`-Felder geklont werden.
impl<R: Request> Clone for Next<R> {
    fn clone(&self) -> Self {
        Self {
            handler: Arc::clone(&self.handler),
            behaviors: self.behaviors.clone(),
        }
    }
}

impl<R: Request> fmt::Debug for Next<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Next").finish_non_exhaustive()
    }
}
