//! Die Glieder der Behavior-Kette, die nur diese Crate baut.

use std::fmt;
use std::rc::Rc;

use mediatr_provider::Provider;

use crate::PipelineBehavior;
use crate::Request;
use crate::wrappers::{BehaviorWrapper, wrap_behavior};

/// Die Kette der Behaviors eines Mediators: `()` oder ein Paar aus der Kette davor und einem weiteren Behavior.
///
/// Öffentlich, weil [`Sender`](crate::Sender) für [`Mediator`](crate::Mediator) es verlangt;
/// in einem privaten Modul, damit nur diese Crate es implementiert.
///
/// # Vertrag
///
/// `wrap` liefert je Behavior der Kette, das Requests vom Typ `R` umschließt und dessen Platz in `active` steht,
/// genau ein gelöschtes Behavior mit seiner Stufe, in Registrierungsreihenfolge, und löst keine Panic aus.
pub trait Behaviors<R: Request>: 'static {
    /// Die Behaviors der Kette für `R` mit gelöschtem Typ und ihrer Stufe, deren Platz in `active` steht, in Registrierungsreihenfolge.
    fn wrap(&self, active: &[Slot]) -> Vec<(Stage, Rc<BehaviorWrapper<R>>)>;
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
pub struct OpenBehavior<B> {
    pub(crate) behavior: Rc<B>,
    pub(crate) slot: Slot,
    pub(crate) stage: Stage,
}

/// Ein Vorverarbeiter als Behavior: `B` sieht den Request, dann geht dieser weiter.
pub struct RequestPreProcessorBehavior<B>(pub(crate) B);

/// Ein Nachverarbeiter als Behavior: `B` sieht eine Kopie des Requests mit der Antwort des Handlers.
pub struct RequestPostProcessorBehavior<B>(pub(crate) B);

/// Ein Behavior nur für einen Request-Typ, als Glied der Kette.
/// Ein eigener [`Provider`] hält es als [`BehaviorWrapper`] dieses Typs, damit `wrap` es nur für diesen Typ findet.
#[derive(Debug, Clone)]
pub struct Closed {
    pub(crate) provider: Provider,
    pub(crate) slot: Slot,
    pub(crate) stage: Stage,
}

// Von Hand statt `derive`: `derive(Clone)` verlangte `B: Clone`, obwohl nur der `Rc` geklont wird.
impl<B> Clone for OpenBehavior<B> {
    fn clone(&self) -> Self {
        Self {
            behavior: Rc::clone(&self.behavior),
            slot: self.slot,
            stage: self.stage,
        }
    }
}

// Von Hand statt `derive`: `derive(Debug)` verlangte `B: Debug`.
impl<B> fmt::Debug for OpenBehavior<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenBehavior")
            .field("slot", &self.slot)
            .field("stage", &self.stage)
            .finish_non_exhaustive()
    }
}

// Von Hand statt `derive`: `derive(Debug)` verlangte `B: Debug`.
impl<B> fmt::Debug for RequestPreProcessorBehavior<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestPreProcessorBehavior")
            .finish_non_exhaustive()
    }
}

// Von Hand statt `derive`: `derive(Debug)` verlangte `B: Debug`.
impl<B> fmt::Debug for RequestPostProcessorBehavior<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestPostProcessorBehavior")
            .finish_non_exhaustive()
    }
}

impl<R: Request> Behaviors<R> for () {
    fn wrap(&self, _active: &[Slot]) -> Vec<(Stage, Rc<BehaviorWrapper<R>>)> {
        Vec::new()
    }
}

impl<R, P, B> Behaviors<R> for (P, OpenBehavior<B>)
where
    R: Request + 'static,
    P: Behaviors<R>,
    B: PipelineBehavior<R> + 'static,
{
    fn wrap(&self, active: &[Slot]) -> Vec<(Stage, Rc<BehaviorWrapper<R>>)> {
        let mut behaviors = self.0.wrap(active);
        if active.contains(&self.1.slot) {
            behaviors.push((
                self.1.stage,
                Rc::new(wrap_behavior(Rc::clone(&self.1.behavior))),
            ));
        }
        behaviors
    }
}

impl<R: Request + 'static, P: Behaviors<R>> Behaviors<R> for (P, Closed) {
    fn wrap(&self, active: &[Slot]) -> Vec<(Stage, Rc<BehaviorWrapper<R>>)> {
        let mut behaviors = self.0.wrap(active);
        if active.contains(&self.1.slot) {
            behaviors.extend(
                self.1
                    .provider
                    .get::<BehaviorWrapper<R>>()
                    .map(|behavior| (self.1.stage, behavior)),
            );
        }
        behaviors
    }
}
