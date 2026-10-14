//! Behaviors, die den Handler eines Requests umschließen.

use std::fmt;
use std::future::Future;
use std::rc::Rc;

use crate::Request;
use crate::erased::{ErasedBehavior, ErasedHandler, erase_behavior};
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

pub(crate) use sealed::Behaviors;

mod sealed {
    use std::rc::Rc;

    use crate::Request;
    use crate::erased::ErasedBehavior;

    /// Ein Tupel von Behaviors, die alle Requests vom Typ `R` umschließen.
    ///
    /// Öffentlich, weil [`Sender`](crate::Sender) für [`Mediator`](crate::Mediator) es verlangt;
    /// in einem privaten Modul, damit nur diese Crate es implementiert.
    ///
    /// # Vertrag
    ///
    /// `erase` liefert genau ein gelöschtes Behavior je Tupel-Element, in Tupel-Reihenfolge, und löst keine Panic aus.
    pub trait Behaviors<R: Request>: 'static {
        /// Die Behaviors aus `behaviors` mit gelöschtem Typ, in Tupel-Reihenfolge.
        fn erase(behaviors: &Rc<Self>) -> Vec<Rc<ErasedBehavior<R>>>;
    }
}

impl<R: Request> Behaviors<R> for () {
    fn erase(_behaviors: &Rc<Self>) -> Vec<Rc<ErasedBehavior<R>>> {
        Vec::new()
    }
}

/// Implementiert [`Behaviors`] für ein Tupel aus Behaviors.
/// Ein Makro, weil weder Funktion noch Generic über die Länge eines Tupels abstrahieren.
macro_rules! behaviors_for_tuple {
    ($($index:tt $behavior:ident),+) => {
        impl<R, $($behavior),+> Behaviors<R> for ($($behavior,)+)
        where
            R: Request + 'static,
            $($behavior: PipelineBehavior<R> + 'static,)+
        {
            fn erase(behaviors: &Rc<Self>) -> Vec<Rc<ErasedBehavior<R>>> {
                vec![$(Rc::new(erase_behavior(Rc::clone(behaviors), |all: &Self| &all.$index))),+]
            }
        }
    };
}

behaviors_for_tuple!(0 A);
behaviors_for_tuple!(0 A, 1 B);

/// Der Rest der Pipeline hinter einem Behavior; [`Next::run`] schickt einen Request hinein.
pub struct Next<R: Request> {
    handler: Rc<ErasedHandler<R>>,
    /// Die noch ausstehenden Behaviors, das innerste zuerst; `run` nimmt das letzte, also äußerste.
    behaviors: Vec<Rc<ErasedBehavior<R>>>,
}

impl<R: Request> Next<R> {
    /// Pipeline aus `behaviors`, in der Reihenfolge ihrer Registrierung, und `handler`; das erste Behavior liegt außen.
    pub(crate) fn new(
        handler: Rc<ErasedHandler<R>>,
        mut behaviors: Vec<Rc<ErasedBehavior<R>>>,
    ) -> Self {
        behaviors.reverse();
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
