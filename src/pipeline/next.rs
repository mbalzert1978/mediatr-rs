//! Der Rest der Pipeline hinter einem Behavior.

use std::fmt;
use std::rc::Rc;

use crate::Request;
use crate::erased::{ErasedBehavior, ErasedHandler};
use crate::pipeline::Stage;
use crate::request::HandlerResult;

/// Der Rest der Pipeline hinter einem Behavior; [`Next::run`] schickt einen Request hinein.
pub struct Next<R: Request> {
    handler: Rc<ErasedHandler<R>>,
    /// Die noch ausstehenden Behaviors, das innerste zuerst; `run` nimmt das letzte, also äußerste.
    behaviors: Vec<Rc<ErasedBehavior<R>>>,
}

impl<R: Request> Next<R> {
    /// Pipeline aus `behaviors`, in der Reihenfolge ihrer Registrierung, und `handler`; jedes liegt, wo seine [`Stage`] sagt.
    pub(crate) fn new(
        handler: Rc<ErasedHandler<R>>,
        behaviors: &[(Stage, Rc<ErasedBehavior<R>>)],
    ) -> Self {
        let at = |wanted: Stage| {
            behaviors
                .iter()
                .filter(move |(stage, _)| *stage == wanted)
                .map(|(_, behavior)| Rc::clone(behavior))
        };
        Self {
            behaviors: at(Stage::Around)
                .rev()
                .chain(at(Stage::After))
                .chain(at(Stage::Before).rev())
                .collect(),
            handler,
        }
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

// Von Hand statt `derive`: `derive(Clone)` verlangte `R: Clone`, obwohl nur die `Rc`-Felder geklont werden.
impl<R: Request> Clone for Next<R> {
    fn clone(&self) -> Self {
        Self {
            handler: Rc::clone(&self.handler),
            behaviors: self.behaviors.clone(),
        }
    }
}

impl<R: Request> fmt::Debug for Next<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Next").finish_non_exhaustive()
    }
}
