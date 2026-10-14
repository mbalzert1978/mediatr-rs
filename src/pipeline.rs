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

pub(crate) use sealed::{Behaviors, Closed, Open, Slot};

mod sealed {
    use std::fmt;
    use std::rc::Rc;

    use mediatr_provider::Provider;

    use crate::Request;
    use crate::erased::ErasedBehavior;

    /// Die Kette der Behaviors eines Mediators: `()` oder ein Paar aus der Kette davor und einem weiteren Behavior.
    ///
    /// Öffentlich, weil [`Sender`](crate::Sender) für [`Mediator`](crate::Mediator) es verlangt;
    /// in einem privaten Modul, damit nur diese Crate es implementiert.
    ///
    /// # Vertrag
    ///
    /// `erase` liefert je Behavior der Kette, das Requests vom Typ `R` umschließt und dessen Platz in `active` steht,
    /// genau ein gelöschtes Behavior, in Registrierungsreihenfolge, und löst keine Panic aus.
    pub trait Behaviors<R: Request>: 'static {
        /// Die Behaviors der Kette für `R` mit gelöschtem Typ, deren Platz in `active` steht, in Registrierungsreihenfolge.
        fn erase(&self, active: &[Slot]) -> Vec<Rc<ErasedBehavior<R>>>;
    }

    /// Der Platz eines Behaviors in der Kette; `send` lässt nur die Plätze laufen, die der Provider des Mediators behalten hat.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Slot(pub(crate) usize);

    /// Ein Behavior für alle Request-Typen, als Glied der Kette.
    pub struct Open<B> {
        pub(crate) behavior: Rc<B>,
        pub(crate) slot: Slot,
    }

    /// Ein Behavior nur für einen Request-Typ, als Glied der Kette.
    /// Ein eigener [`Provider`] hält es als [`ErasedBehavior`] dieses Typs, damit `erase` es nur für diesen Typ findet.
    #[derive(Debug, Clone)]
    pub struct Closed {
        pub(crate) provider: Provider,
        pub(crate) slot: Slot,
    }

    // Von Hand statt `derive`: `derive(Clone)` verlangte `B: Clone`, obwohl nur der `Rc` geklont wird.
    impl<B> Clone for Open<B> {
        fn clone(&self) -> Self {
            Self {
                behavior: Rc::clone(&self.behavior),
                slot: self.slot,
            }
        }
    }

    // Von Hand statt `derive`: `derive(Debug)` verlangte `B: Debug`.
    impl<B> fmt::Debug for Open<B> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Open")
                .field("slot", &self.slot)
                .finish_non_exhaustive()
        }
    }
}

impl<R: Request> Behaviors<R> for () {
    fn erase(&self, _active: &[Slot]) -> Vec<Rc<ErasedBehavior<R>>> {
        Vec::new()
    }
}

impl<R, P, B> Behaviors<R> for (P, Open<B>)
where
    R: Request + 'static,
    P: Behaviors<R>,
    B: PipelineBehavior<R> + 'static,
{
    fn erase(&self, active: &[Slot]) -> Vec<Rc<ErasedBehavior<R>>> {
        let mut behaviors = self.0.erase(active);
        if active.contains(&self.1.slot) {
            behaviors.push(Rc::new(erase_behavior(Rc::clone(&self.1.behavior))));
        }
        behaviors
    }
}

impl<R: Request + 'static, P: Behaviors<R>> Behaviors<R> for (P, Closed) {
    fn erase(&self, active: &[Slot]) -> Vec<Rc<ErasedBehavior<R>>> {
        let mut behaviors = self.0.erase(active);
        if active.contains(&self.1.slot) {
            behaviors.extend(self.1.provider.get::<ErasedBehavior<R>>());
        }
        behaviors
    }
}

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
