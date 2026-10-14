//! Der Builder, aus dem ein Mediator entsteht.

use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;

use mediatr_provider::{Provider, ProviderBuilder, Registration};

use crate::pipeline::{
    Closed, OpenBehavior, RequestPostProcessorBehavior, RequestPreProcessorBehavior, Slot, Stage,
};
use crate::wrappers::{
    BehaviorWrapper, wrap_behavior, wrap_notification_handler, wrap_request_handler,
};
use crate::{
    ForeachAwaitPublisher, Handler, Mediator, Notification, NotificationHandler, PipelineBehavior,
    Request, RequestHandler, RequestPostProcessor, RequestPreProcessor,
};

/// Sammelt Behaviors und Handler und baut daraus einen [`Mediator`] mit dem Notification-Publisher `S` und der Kette `P` der Behaviors.
///
/// `F` sagt, ob schon ein Handler registriert ist; Behaviors nimmt der Builder nur davor,
/// weil jeder Handler die Kette der Behaviors schon in seinem Typ trägt.
pub struct MediatorBuilder<S = ForeachAwaitPublisher, P = (), F = NoHandlers> {
    provider: ProviderBuilder,
    behaviors: P,
    /// Wie viele Behaviors die Kette hat; der Platz des nächsten.
    slots: usize,
    types: PhantomData<(S, F)>,
}

/// [`MediatorBuilder`] ohne Handler; er nimmt noch Behaviors.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct NoHandlers;

/// [`MediatorBuilder`] mit mindestens einem Handler; er nimmt keine Behaviors mehr.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct WithHandlers;

// Von Hand statt `derive`: `derive(Debug)` verlangte `S: Debug`; `PhantomData` nennt den Publisher auch ohne.
impl<S, P: fmt::Debug, F> fmt::Debug for MediatorBuilder<S, P, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MediatorBuilder")
            .field("provider", &self.provider)
            .field("behaviors", &self.behaviors)
            .field("slots", &self.slots)
            .field("types", &self.types)
            .finish()
    }
}

// Von Hand statt `derive`: `derive(Clone)` verlangte `S: Clone`, obwohl nur `PhantomData` von `S` abhängt.
impl<S, P: Clone, F> Clone for MediatorBuilder<S, P, F> {
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            behaviors: self.behaviors.clone(),
            slots: self.slots,
            types: PhantomData,
        }
    }
}

impl<S> Default for MediatorBuilder<S> {
    fn default() -> Self {
        Self {
            provider: ProviderBuilder::default(),
            behaviors: (),
            slots: 0,
            types: PhantomData,
        }
    }
}

impl<S, P> MediatorBuilder<S, P, NoHandlers> {
    /// Registriert `behavior` für alle Request-Typen; es liegt innerhalb der zuvor registrierten Behaviors.
    ///
    /// [`Sender::send`](crate::Sender::send) verlangt, dass `behavior` [`PipelineBehavior`] für den Request-Typ ist.
    /// Ist ein Behavior vom Typ `B` schon registriert, läuft nur das erste, an seinem Platz.
    #[must_use]
    pub fn open_behavior<B: 'static>(
        self,
        behavior: B,
    ) -> MediatorBuilder<S, (P, OpenBehavior<B>)> {
        self.open(behavior, Stage::Around)
    }

    /// Registriert `behavior` nur für Requests vom Typ `R`; es liegt innerhalb der zuvor registrierten Behaviors.
    #[must_use]
    pub fn behavior<R: Request + 'static>(
        self,
        behavior: impl PipelineBehavior<R> + 'static,
    ) -> MediatorBuilder<S, (P, Closed)> {
        self.closed::<R>(behavior, Stage::Around)
    }

    /// Registriert `pre_processor` für alle Request-Typen; er läuft vor allen Behaviors.
    ///
    /// [`Sender::send`](crate::Sender::send) verlangt, dass `pre_processor` [`RequestPreProcessor`](crate::RequestPreProcessor) für den Request-Typ ist.
    #[must_use]
    pub fn open_request_pre_processor<B: 'static>(
        self,
        pre_processor: B,
    ) -> MediatorBuilder<S, (P, OpenBehavior<RequestPreProcessorBehavior<B>>)> {
        self.open(RequestPreProcessorBehavior(pre_processor), Stage::Before)
    }

    /// Registriert `post_processor` für alle Request-Typen; er läuft nach allen Behaviors.
    ///
    /// [`Sender::send`](crate::Sender::send) verlangt, dass `post_processor` [`RequestPostProcessor`](crate::RequestPostProcessor) für den Request-Typ ist
    /// und der Request [`Clone`] ist, denn der Handler verbraucht ihn.
    #[must_use]
    pub fn open_request_post_processor<B: 'static>(
        self,
        post_processor: B,
    ) -> MediatorBuilder<S, (P, OpenBehavior<RequestPostProcessorBehavior<B>>)> {
        self.open(RequestPostProcessorBehavior(post_processor), Stage::After)
    }

    /// Registriert `pre_processor` nur für Requests vom Typ `R`; er läuft vor allen Behaviors.
    #[must_use]
    pub fn request_pre_processor<R: Request + 'static>(
        self,
        pre_processor: impl RequestPreProcessor<R> + 'static,
    ) -> MediatorBuilder<S, (P, Closed)> {
        self.closed::<R>(RequestPreProcessorBehavior(pre_processor), Stage::Before)
    }

    /// Registriert `post_processor` nur für Requests vom Typ `R`; er läuft nach allen Behaviors.
    #[must_use]
    pub fn request_post_processor<R: Request + Clone + 'static>(
        self,
        post_processor: impl RequestPostProcessor<R> + 'static,
    ) -> MediatorBuilder<S, (P, Closed)> {
        self.closed::<R>(RequestPostProcessorBehavior(post_processor), Stage::After)
    }

    /// Builder, dessen Kette zusätzlich mit `behavior` für alle Request-Typen auf der Stufe `stage` endet.
    fn open<B: 'static>(
        self,
        behavior: B,
        stage: Stage,
    ) -> MediatorBuilder<S, (P, OpenBehavior<B>)> {
        let slot = Slot(self.slots);
        self.with_behavior(
            OpenBehavior {
                behavior: Rc::new(behavior),
                slot,
                stage,
            },
            Registration::instance(slot).unique_by::<B>(),
        )
    }

    /// Builder, dessen Kette zusätzlich mit `behavior` nur für Requests vom Typ `R` auf der Stufe `stage` endet.
    fn closed<R: Request + 'static>(
        self,
        behavior: impl PipelineBehavior<R> + 'static,
        stage: Stage,
    ) -> MediatorBuilder<S, (P, Closed)> {
        let slot = Slot(self.slots);
        let unique = unique_for::<R, _>(slot, &behavior);
        let wrapper: BehaviorWrapper<R> = wrap_behavior(Rc::new(behavior));
        self.with_behavior(
            Closed {
                provider: Provider::builder()
                    .register(Registration::instance(wrapper))
                    .build(),
                slot,
                stage,
            },
            unique,
        )
    }

    /// Builder, dessen Kette der Behaviors zusätzlich mit `link` endet; `slot` meldet dessen Platz an.
    fn with_behavior<L>(self, link: L, slot: Registration<Slot>) -> MediatorBuilder<S, (P, L)> {
        MediatorBuilder {
            provider: self.provider.register(slot),
            behaviors: (self.behaviors, link),
            slots: self.slots.saturating_add(1),
            types: PhantomData,
        }
    }
}

impl<S, P, F> MediatorBuilder<S, P, F> {
    /// Registriert `handler` für Requests vom Typ `R`; ein früherer Handler für `R` fällt weg.
    #[must_use]
    pub fn register<R, H>(self, handler: Handler<H, S, P>) -> MediatorBuilder<S, P, WithHandlers>
    where
        R: Request + 'static,
        H: RequestHandler<R> + 'static,
    {
        self.with_registration(handler.registration.map(wrap_request_handler))
    }

    /// Registriert `handler` für Notifications vom Typ `N`; ist für `N` schon ein Handler vom Typ `H` registriert, fällt `handler` weg.
    #[must_use]
    pub fn subscribe<N, H>(self, handler: Handler<H, S, P>) -> MediatorBuilder<S, P, WithHandlers>
    where
        N: Notification + 'static,
        H: NotificationHandler<N> + 'static,
    {
        self.with_registration(
            handler
                .registration
                .map(wrap_notification_handler)
                .unique_by::<H>(),
        )
    }

    /// Baut den Mediator mit allen registrierten Behaviors und Handlern.
    #[must_use]
    pub fn build(self) -> Mediator<S, P>
    where
        P: 'static,
    {
        Mediator::new(
            self.provider
                .register(Registration::instance(self.behaviors))
                .build(),
        )
    }

    /// Builder, dessen Provider zusätzlich `registration` kennt.
    fn with_registration<T: 'static>(
        self,
        registration: Registration<T>,
    ) -> MediatorBuilder<S, P, WithHandlers> {
        MediatorBuilder {
            provider: self.provider.register(registration),
            behaviors: self.behaviors,
            slots: self.slots,
            types: PhantomData,
        }
    }
}

/// Meldet `slot` an; der Provider behält nur die erste Meldung je Paar aus Request-Typ `R` und Behavior-Typ `B`.
// Eigene Funktion, weil `closed` den Behavior-Typ nicht beim Namen kennt; `_behavior` liefert ihn.
fn unique_for<R: 'static, B: 'static>(slot: Slot, _behavior: &B) -> Registration<Slot> {
    Registration::instance(slot).unique_by::<(R, B)>()
}
