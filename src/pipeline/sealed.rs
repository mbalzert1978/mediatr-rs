//! Die Glieder der Behavior-Kette, die nur diese Crate baut.

use std::fmt;
use std::sync::Arc;

use mediatr_provider::Provider;

use crate::wrappers::{
    BehaviorWrapper, StreamBehaviorWrapper, wrap_behavior, wrap_stream_behavior,
};
use crate::{PipelineBehavior, Request, StreamPipelineBehavior, StreamRequest};

/// Die Kette der Behaviors eines Mediators: `()` oder ein Paar aus der Kette davor und einem weiteren Behavior.
///
/// Öffentlich, weil [`MediatorBuilder::register`](crate::MediatorBuilder::register) es verlangt;
/// in einem privaten Modul, damit nur diese Crate es implementiert.
///
/// # Vertrag
///
/// `wrap` liefert je Behavior der Kette, das Requests vom Typ `R` umschließt und dessen Platz in `active` steht,
/// genau einen [`BehaviorWrapper`] mit seiner Stufe, in Registrierungsreihenfolge, und löst keine Panic aus.
pub trait Behaviors<R: Request>: 'static {
    /// Die Behaviors der Kette für `R` als [`BehaviorWrapper`] mit ihrer Stufe, deren Platz in `active` steht, in Registrierungsreihenfolge.
    fn wrap(&self, active: &[Slot]) -> Vec<(Stage, Arc<BehaviorWrapper<R>>)>;
}

/// Die Kette der Behaviors eines Mediators, gelesen für Stream-Requests: nur ihre Stream-Behaviors zählen.
///
/// Öffentlich, weil [`MediatorBuilder::register_stream`](crate::MediatorBuilder::register_stream) es verlangt;
/// in einem privaten Modul, damit nur diese Crate es implementiert.
///
/// # Vertrag
///
/// `wrap` liefert je Stream-Behavior der Kette, das Stream-Requests vom Typ `R` umschließt und dessen Platz in `active` steht,
/// genau einen [`StreamBehaviorWrapper`], in Registrierungsreihenfolge, und löst keine Panic aus.
pub trait StreamBehaviors<R: StreamRequest>: 'static {
    /// Die Stream-Behaviors der Kette für `R` als [`StreamBehaviorWrapper`], deren Platz in `active` steht, in Registrierungsreihenfolge.
    fn wrap(&self, active: &[Slot]) -> Vec<Arc<StreamBehaviorWrapper<R>>>;
}

/// Wo ein Behavior in der Pipeline liegt; die Varianten stehen von außen nach innen, `Ord` folgt dieser Reihenfolge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    /// Außerhalb aller anderen; reagiert nach ihnen in Registrierungsreihenfolge.
    ErrorAction,
    /// Innerhalb von `ErrorAction`, außerhalb aller anderen; reagiert vor `ErrorAction` in Registrierungsreihenfolge.
    ErrorHandler,
    /// Innerhalb von `ErrorHandler`, außerhalb aller anderen; läuft vor ihnen in Registrierungsreihenfolge.
    Before,
    /// Innerhalb von `Before`, außerhalb von `Around`; läuft nach dem Rest in Registrierungsreihenfolge.
    After,
    /// Direkt um den Handler, das zuerst registrierte außen.
    Around,
}

/// Der Platz eines Behaviors in der Kette; `register` nimmt nur die Plätze in die Pipeline, die der Builder behalten hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Slot(pub(crate) usize);

/// Ein Behavior für alle Request-Typen, als Glied der Kette.
pub struct OpenBehavior<B> {
    pub(crate) behavior: Arc<B>,
    pub(crate) slot: Slot,
    pub(crate) stage: Stage,
}

/// Ein Stream-Behavior für alle Stream-Request-Typen, als Glied der Kette.
pub struct OpenStreamBehavior<B> {
    pub(crate) behavior: Arc<B>,
    pub(crate) slot: Slot,
}

/// Ein Stream-Behavior nur für einen Stream-Request-Typ, als Glied der Kette.
/// Ein eigener [`Provider`] hält es als [`StreamBehaviorWrapper`] dieses Typs, damit `wrap` es nur für diesen Typ findet.
#[derive(Debug, Clone)]
pub struct ClosedStreamBehavior {
    pub(crate) provider: Provider,
    pub(crate) slot: Slot,
}

/// Ein Vorverarbeiter als Behavior: `B` sieht den Request, dann geht dieser weiter.
#[derive(Debug)]
pub struct RequestPreProcessorBehavior<B>(pub(crate) B);

/// Ein Nachverarbeiter als Behavior: `B` sieht eine Kopie des Requests mit der Antwort der inneren Pipeline (Behaviors und Handler).
#[derive(Debug)]
pub struct RequestPostProcessorBehavior<B>(pub(crate) B);

/// Eine Fehler-Aktion als Behavior: `B` sieht eine Kopie des Requests mit dem Fehler der inneren Pipeline (Behaviors und Handler).
#[derive(Debug)]
pub struct RequestErrorActionProcessorBehavior<B>(pub(crate) B);

/// Ein Fehler-Handler als Behavior: `B` bekommt eine Kopie des Requests mit dem Fehler der inneren Pipeline (Behaviors und Handler).
#[derive(Debug)]
pub struct RequestErrorProcessorBehavior<B>(pub(crate) B);

/// Ein Behavior nur für einen Request-Typ, als Glied der Kette.
/// Ein eigener [`Provider`] hält es als [`BehaviorWrapper`] dieses Typs, damit `wrap` es nur für diesen Typ findet.
#[derive(Debug, Clone)]
pub struct ClosedBehavior {
    pub(crate) provider: Provider,
    pub(crate) slot: Slot,
    pub(crate) stage: Stage,
}

// Von Hand statt `derive`: `derive(Clone)` verlangte `B: Clone`, obwohl nur der `Arc` geklont wird.
impl<B> Clone for OpenBehavior<B> {
    fn clone(&self) -> Self {
        Self {
            behavior: Arc::clone(&self.behavior),
            slot: self.slot,
            stage: self.stage,
        }
    }
}

// Von Hand statt `derive`: `derive(Clone)` verlangte `B: Clone`, obwohl nur der `Arc` geklont wird.
impl<B> Clone for OpenStreamBehavior<B> {
    fn clone(&self) -> Self {
        Self {
            behavior: Arc::clone(&self.behavior),
            slot: self.slot,
        }
    }
}

// Von Hand statt `derive`: `derive(Debug)` verlangte `B: Debug`.
impl<B> fmt::Debug for OpenStreamBehavior<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenStreamBehavior")
            .field("slot", &self.slot)
            .finish_non_exhaustive()
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

impl<R: Request> Behaviors<R> for () {
    fn wrap(&self, _active: &[Slot]) -> Vec<(Stage, Arc<BehaviorWrapper<R>>)> {
        Vec::new()
    }
}

impl<R, P, B> Behaviors<R> for (P, OpenBehavior<B>)
where
    R: Request + 'static,
    P: Behaviors<R>,
    B: PipelineBehavior<R> + Send + Sync + 'static,
{
    fn wrap(&self, active: &[Slot]) -> Vec<(Stage, Arc<BehaviorWrapper<R>>)> {
        let mut behaviors = self.0.wrap(active);
        if active.contains(&self.1.slot) {
            behaviors.push((
                self.1.stage,
                Arc::new(wrap_behavior(Arc::clone(&self.1.behavior))),
            ));
        }
        behaviors
    }
}

impl<R: Request + 'static, P: Behaviors<R>> Behaviors<R> for (P, ClosedBehavior) {
    fn wrap(&self, active: &[Slot]) -> Vec<(Stage, Arc<BehaviorWrapper<R>>)> {
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

impl<R: Request + 'static, P: Behaviors<R>, B: 'static> Behaviors<R>
    for (P, OpenStreamBehavior<B>)
{
    fn wrap(&self, active: &[Slot]) -> Vec<(Stage, Arc<BehaviorWrapper<R>>)> {
        self.0.wrap(active)
    }
}

impl<R: StreamRequest> StreamBehaviors<R> for () {
    fn wrap(&self, _active: &[Slot]) -> Vec<Arc<StreamBehaviorWrapper<R>>> {
        Vec::new()
    }
}

impl<R: StreamRequest, P: StreamBehaviors<R>, B: 'static> StreamBehaviors<R>
    for (P, OpenBehavior<B>)
{
    fn wrap(&self, active: &[Slot]) -> Vec<Arc<StreamBehaviorWrapper<R>>> {
        self.0.wrap(active)
    }
}

impl<R: StreamRequest, P: StreamBehaviors<R>> StreamBehaviors<R> for (P, ClosedBehavior) {
    fn wrap(&self, active: &[Slot]) -> Vec<Arc<StreamBehaviorWrapper<R>>> {
        self.0.wrap(active)
    }
}

impl<R, P, B> StreamBehaviors<R> for (P, OpenStreamBehavior<B>)
where
    R: StreamRequest + 'static,
    P: StreamBehaviors<R>,
    B: StreamPipelineBehavior<R> + Send + Sync + 'static,
{
    fn wrap(&self, active: &[Slot]) -> Vec<Arc<StreamBehaviorWrapper<R>>> {
        let mut behaviors = self.0.wrap(active);
        if active.contains(&self.1.slot) {
            behaviors.push(Arc::new(wrap_stream_behavior(Arc::clone(&self.1.behavior))));
        }
        behaviors
    }
}

impl<R: Request + 'static, P: Behaviors<R>> Behaviors<R> for (P, ClosedStreamBehavior) {
    fn wrap(&self, active: &[Slot]) -> Vec<(Stage, Arc<BehaviorWrapper<R>>)> {
        self.0.wrap(active)
    }
}

impl<R: StreamRequest + 'static, P: StreamBehaviors<R>> StreamBehaviors<R>
    for (P, ClosedStreamBehavior)
{
    fn wrap(&self, active: &[Slot]) -> Vec<Arc<StreamBehaviorWrapper<R>>> {
        let mut behaviors = self.0.wrap(active);
        if active.contains(&self.1.slot) {
            behaviors.extend(self.1.provider.get::<StreamBehaviorWrapper<R>>());
        }
        behaviors
    }
}
