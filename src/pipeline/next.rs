//! Der Rest der Pipeline hinter einem Behavior.

use std::cmp::Reverse;
use std::fmt;
use std::rc::Rc;

use crate::Request;
use crate::pipeline::Stage;
use crate::request::HandlerResult;
use crate::wrappers::{BehaviorWrapper, RequestHandlerWrapper};

/// Der Rest der Pipeline hinter einem Behavior; [`Next::run`] schickt einen Request hinein.
pub struct Next<R: Request> {
    handler: Rc<RequestHandlerWrapper<R>>,
    /// Die noch ausstehenden Behaviors, das innerste zuerst; `run` nimmt das letzte, also äußerste.
    behaviors: Vec<(Stage, Rc<BehaviorWrapper<R>>)>,
}

impl<R: Request> Next<R> {
    /// Pipeline aus `behaviors`, in der Reihenfolge ihrer Registrierung, und `handler`; jedes liegt, wo seine [`Stage`] sagt.
    pub(crate) fn new(
        handler: Rc<RequestHandlerWrapper<R>>,
        mut behaviors: Vec<(Stage, Rc<BehaviorWrapper<R>>)>,
    ) -> Self {
        // Innerste Stufe zuerst; die Sortierung ist stabil, in jeder Stufe bleibt die Registrierungsreihenfolge.
        behaviors.sort_by_key(|(stage, _)| Reverse(*stage));
        // Bei `Around` und `Before` liegt das zuerst registrierte außen, also hinten.
        for stage in [Stage::Around, Stage::Before] {
            let start = behaviors.partition_point(|(other, _)| *other > stage);
            let end = behaviors.partition_point(|(other, _)| *other >= stage);
            behaviors
                .get_mut(start..end)
                .into_iter()
                .for_each(<[_]>::reverse);
        }
        Self { handler, behaviors }
    }

    /// Schickt `request` an den nächsten Schritt und liefert dessen Antwort.
    ///
    /// # Errors
    ///
    /// Den Fehler des nächsten Schritts.
    pub async fn run(mut self, request: R) -> HandlerResult<R> {
        match self.behaviors.pop() {
            Some((_, behavior)) => behavior(request, self).await,
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
