//! Die Pipeline um den Handler eines Requests.

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

/// Läuft vor dem Handler von Requests vom Typ `R` und sieht den Request.
///
/// # Vertrag
///
/// `process` endet für jeden Request mit `Ok` oder einem Fehler und löst keine Panic aus.
pub trait RequestPreProcessor<R: Request> {
    /// Sieht `request`, bevor der Handler ihn bekommt.
    ///
    /// # Errors
    ///
    /// `R::Error`, wenn `request` nicht weiter darf; dann läuft der Handler nicht, und `send` liefert diesen Fehler.
    fn process(&self, request: &R) -> impl Future<Output = Result<(), R::Error>>;
}

/// Läuft nach dem Handler von Requests vom Typ `R` und sieht Request samt Antwort.
///
/// # Vertrag
///
/// `process` endet für jeden Request mit `Ok` oder einem Fehler und löst keine Panic aus.
pub trait RequestPostProcessor<R: Request> {
    /// Sieht `request` mit der `response` des Handlers.
    ///
    /// # Errors
    ///
    /// `R::Error`, wenn `response` nicht hinaus darf; dann liefert `send` diesen Fehler statt der Antwort.
    fn process(
        &self,
        request: &R,
        response: &R::Response,
    ) -> impl Future<Output = Result<(), R::Error>>;
}

pub(crate) use sealed::{Behaviors, Closed, Open, PostProcess, PreProcess, Slot, Stage};

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
    /// genau ein gelöschtes Behavior mit seiner Stufe, in Registrierungsreihenfolge, und löst keine Panic aus.
    pub trait Behaviors<R: Request>: 'static {
        /// Die Behaviors der Kette für `R` mit gelöschtem Typ und ihrer Stufe, deren Platz in `active` steht, in Registrierungsreihenfolge.
        fn erase(&self, active: &[Slot]) -> Vec<(Stage, Rc<ErasedBehavior<R>>)>;
    }

    /// Wo ein Behavior in der Pipeline liegt.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Stage {
        /// Außerhalb aller anderen; läuft vor ihnen in Registrierungsreihenfolge.
        Before,
        /// Innerhalb von `Before`, außerhalb von `Around`; läuft nach dem Rest in Registrierungsreihenfolge.
        After,
        /// Direkt um den Handler, das zuerst registrierte außen.
        Around,
    }

    /// Der Platz eines Behaviors in der Kette; `send` lässt nur die Plätze laufen, die der Provider des Mediators behalten hat.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Slot(pub(crate) usize);

    /// Ein Behavior für alle Request-Typen, als Glied der Kette.
    pub struct Open<B> {
        pub(crate) behavior: Rc<B>,
        pub(crate) slot: Slot,
        pub(crate) stage: Stage,
    }

    /// Ein Vorverarbeiter als Behavior: `B` sieht den Request, dann geht dieser weiter.
    pub struct PreProcess<B>(pub(crate) B);

    /// Ein Nachverarbeiter als Behavior: `B` sieht eine Kopie des Requests mit der Antwort des Handlers.
    pub struct PostProcess<B>(pub(crate) B);

    /// Ein Behavior nur für einen Request-Typ, als Glied der Kette.
    /// Ein eigener [`Provider`] hält es als [`ErasedBehavior`] dieses Typs, damit `erase` es nur für diesen Typ findet.
    #[derive(Debug, Clone)]
    pub struct Closed {
        pub(crate) provider: Provider,
        pub(crate) slot: Slot,
        pub(crate) stage: Stage,
    }

    // Von Hand statt `derive`: `derive(Clone)` verlangte `B: Clone`, obwohl nur der `Rc` geklont wird.
    impl<B> Clone for Open<B> {
        fn clone(&self) -> Self {
            Self {
                behavior: Rc::clone(&self.behavior),
                slot: self.slot,
                stage: self.stage,
            }
        }
    }

    // Von Hand statt `derive`: `derive(Debug)` verlangte `B: Debug`.
    impl<B> fmt::Debug for Open<B> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Open")
                .field("slot", &self.slot)
                .field("stage", &self.stage)
                .finish_non_exhaustive()
        }
    }

    // Von Hand statt `derive`: `derive(Debug)` verlangte `B: Debug`.
    impl<B> fmt::Debug for PreProcess<B> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("PreProcess").finish_non_exhaustive()
        }
    }

    // Von Hand statt `derive`: `derive(Debug)` verlangte `B: Debug`.
    impl<B> fmt::Debug for PostProcess<B> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("PostProcess").finish_non_exhaustive()
        }
    }
}

impl<R: Request, B: RequestPreProcessor<R>> PipelineBehavior<R> for PreProcess<B> {
    async fn handle(&self, request: R, next: Next<R>) -> HandlerResult<R> {
        self.0.process(&request).await?;
        next.run(request).await
    }
}

impl<R: Request + Clone, B: RequestPostProcessor<R>> PipelineBehavior<R> for PostProcess<B> {
    async fn handle(&self, request: R, next: Next<R>) -> HandlerResult<R> {
        let copy = request.clone();
        let response = next.run(request).await?;
        self.0.process(&copy, &response).await?;
        Ok(response)
    }
}

impl<R: Request> Behaviors<R> for () {
    fn erase(&self, _active: &[Slot]) -> Vec<(Stage, Rc<ErasedBehavior<R>>)> {
        Vec::new()
    }
}

impl<R, P, B> Behaviors<R> for (P, Open<B>)
where
    R: Request + 'static,
    P: Behaviors<R>,
    B: PipelineBehavior<R> + 'static,
{
    fn erase(&self, active: &[Slot]) -> Vec<(Stage, Rc<ErasedBehavior<R>>)> {
        let mut behaviors = self.0.erase(active);
        if active.contains(&self.1.slot) {
            behaviors.push((
                self.1.stage,
                Rc::new(erase_behavior(Rc::clone(&self.1.behavior))),
            ));
        }
        behaviors
    }
}

impl<R: Request + 'static, P: Behaviors<R>> Behaviors<R> for (P, Closed) {
    fn erase(&self, active: &[Slot]) -> Vec<(Stage, Rc<ErasedBehavior<R>>)> {
        let mut behaviors = self.0.erase(active);
        if active.contains(&self.1.slot) {
            behaviors.extend(
                self.1
                    .provider
                    .get::<ErasedBehavior<R>>()
                    .map(|behavior| (self.1.stage, behavior)),
            );
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
