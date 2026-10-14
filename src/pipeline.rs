//! Behaviors, die den Handler eines Requests umschließen.

use std::fmt;
use std::future::Future;
use std::rc::Rc;

use crate::Request;
use crate::erased::{ErasedBehavior, ErasedHandler};
use crate::request::HandlerResult;

/// Umschließt den Handler von Requests vom Typ `R`: läuft davor und danach und entscheidet, ob es weitergeht.
///
/// # Vertrag
///
/// `handle` liefert für jeden Request genau eine Antwort oder einen Fehler und löst keine Panic aus.
pub trait PipelineBehavior<R: Request> {
    /// Bearbeitet `request`; `next` führt zum nächsten Behavior oder zum Handler.
    ///
    /// # Errors
    ///
    /// `R::Error`, wenn das Behavior oder ein späterer Schritt `request` nicht beantworten kann.
    fn handle(&self, request: R, next: Next<R>) -> impl Future<Output = HandlerResult<R>>;
}

/// Der Rest der Pipeline hinter einem Behavior; [`Next::run`] schickt einen Request hinein.
pub struct Next<R: Request> {
    handler: Rc<ErasedHandler<R>>,
    /// Die noch ausstehenden Behaviors; `run` nimmt das letzte zuerst.
    behaviors: Vec<Rc<ErasedBehavior<R>>>,
}

impl<R: Request> Next<R> {
    /// Pipeline aus `behaviors` und `handler`.
    pub(crate) fn new(
        handler: Rc<ErasedHandler<R>>,
        behaviors: Vec<Rc<ErasedBehavior<R>>>,
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
