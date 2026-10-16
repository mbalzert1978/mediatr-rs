//! Der Builder, aus dem ein Mediator entsteht.

use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use mediatr_provider::{Provider, ProviderBuilder, Registration};

use crate::pipeline::{
    Behaviors, ClosedBehavior, ClosedStreamBehavior, OpenBehavior, OpenStreamBehavior,
    RequestErrorActionProcessorBehavior, RequestErrorProcessorBehavior,
    RequestPostProcessorBehavior, RequestPreProcessorBehavior, Slot, Stage, StreamBehaviors, order,
};
use crate::wrappers::{
    BehaviorWrapper, StreamBehaviorWrapper, wrap_behavior, wrap_notification_handler,
    wrap_request_handler, wrap_stream_behavior, wrap_stream_request_handler,
};
use crate::{
    ForeachAwaitPublisher, Handler, Mediator, Notification, NotificationHandler, PipelineBehavior,
    Request, RequestErrorAction, RequestErrorHandler, RequestHandler, RequestPostProcessor,
    RequestPreProcessor, StreamPipelineBehavior, StreamRequest, StreamRequestHandler,
};

/// Sammelt Behaviors und Handler und baut daraus einen [`Mediator`] mit dem Notification-Publisher `S`; `P` ist die Kette der Behaviors, aus der `register` je Request-Typ die Pipeline baut.
///
/// `F` sagt, ob schon ein Handler registriert ist; Behaviors nimmt der Builder nur davor,
/// weil `register` die Pipeline aus den bis dahin registrierten Behaviors baut.
pub struct MediatorBuilder<S = ForeachAwaitPublisher, P = (), F = NoHandlers> {
    provider: ProviderBuilder,
    behaviors: P,
    /// Wie viele Behaviors die Kette hat; der Platz des nächsten.
    slots: usize,
    /// Die Plätze der Behaviors, die laufen: je Schlüsseltyp nur der zuerst registrierte, in Registrierungsreihenfolge.
    active: Vec<Slot>,
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
            .field("active", &self.active)
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
            active: self.active.clone(),
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
            active: Vec::new(),
            types: PhantomData,
        }
    }
}

impl<S, P> MediatorBuilder<S, P, NoHandlers> {
    /// Registriert `behavior` für alle Request-Typen; es liegt innerhalb der zuvor registrierten Behaviors.
    ///
    /// [`MediatorBuilder::register`] verlangt, dass `behavior` [`PipelineBehavior`] für den Request-Typ ist.
    /// Ist ein Behavior vom Typ `B` schon registriert, läuft nur das erste, an seinem Platz.
    #[must_use]
    pub fn open_behavior<B: Send + Sync + 'static>(
        self,
        behavior: B,
    ) -> MediatorBuilder<S, (P, OpenBehavior<B>)> {
        self.open(behavior, Stage::Around)
    }

    /// Registriert `behavior` nur für Requests vom Typ `R`; es liegt innerhalb der zuvor registrierten Behaviors.
    #[must_use]
    pub fn behavior<R: Request>(
        self,
        behavior: impl PipelineBehavior<R> + 'static,
    ) -> MediatorBuilder<S, (P, ClosedBehavior)> {
        self.closed::<R, _>(behavior, Stage::Around)
    }

    /// Registriert `pre_processor` für alle Request-Typen; er läuft vor allen Behaviors.
    ///
    /// [`MediatorBuilder::register`] verlangt, dass `pre_processor` [`RequestPreProcessor`](crate::RequestPreProcessor) für den Request-Typ ist.
    #[must_use]
    pub fn open_request_pre_processor<B: Send + Sync + 'static>(
        self,
        pre_processor: B,
    ) -> MediatorBuilder<S, (P, OpenBehavior<RequestPreProcessorBehavior<B>>)> {
        self.open(RequestPreProcessorBehavior(pre_processor), Stage::Before)
    }

    /// Registriert `post_processor` für alle Request-Typen; er läuft nach allen Behaviors.
    ///
    /// [`MediatorBuilder::register`] verlangt, dass `post_processor` [`RequestPostProcessor`](crate::RequestPostProcessor) für den Request-Typ ist
    /// und der Request [`Clone`] ist, denn der Handler verbraucht ihn.
    #[must_use]
    pub fn open_request_post_processor<B: Send + Sync + 'static>(
        self,
        post_processor: B,
    ) -> MediatorBuilder<S, (P, OpenBehavior<RequestPostProcessorBehavior<B>>)> {
        self.open(RequestPostProcessorBehavior(post_processor), Stage::After)
    }

    /// Registriert `pre_processor` nur für Requests vom Typ `R`; er läuft vor allen Behaviors.
    #[must_use]
    pub fn request_pre_processor<R: Request>(
        self,
        pre_processor: impl RequestPreProcessor<R> + 'static,
    ) -> MediatorBuilder<S, (P, ClosedBehavior)> {
        self.closed::<R, _>(RequestPreProcessorBehavior(pre_processor), Stage::Before)
    }

    /// Registriert `post_processor` nur für Requests vom Typ `R`; er läuft nach allen Behaviors.
    #[must_use]
    pub fn request_post_processor<R: Request + Clone>(
        self,
        post_processor: impl RequestPostProcessor<R> + 'static,
    ) -> MediatorBuilder<S, (P, ClosedBehavior)> {
        self.closed::<R, _>(RequestPostProcessorBehavior(post_processor), Stage::After)
    }

    /// Registriert `error_action` für alle Request-Typen; sie reagiert auf jeden Fehler, nachdem alle anderen gelaufen sind.
    ///
    /// [`MediatorBuilder::register`] verlangt, dass `error_action` [`RequestErrorAction`](crate::RequestErrorAction) für den Request-Typ ist
    /// und der Request [`Clone`] ist, denn der Handler verbraucht ihn.
    #[must_use]
    pub fn open_request_error_action<B: Send + Sync + 'static>(
        self,
        error_action: B,
    ) -> MediatorBuilder<S, (P, OpenBehavior<RequestErrorActionProcessorBehavior<B>>)> {
        self.open(
            RequestErrorActionProcessorBehavior(error_action),
            Stage::ErrorAction,
        )
    }

    /// Registriert `error_action` nur für Requests vom Typ `R`; sie reagiert auf jeden Fehler, nachdem alle anderen gelaufen sind.
    #[must_use]
    pub fn request_error_action<R: Request + Clone>(
        self,
        error_action: impl RequestErrorAction<R> + 'static,
    ) -> MediatorBuilder<S, (P, ClosedBehavior)> {
        self.closed::<R, _>(
            RequestErrorActionProcessorBehavior(error_action),
            Stage::ErrorAction,
        )
    }

    /// Registriert `error_handler` für alle Request-Typen; er bekommt jeden Fehler, bevor eine Fehler-Aktion ihn sieht.
    ///
    /// [`MediatorBuilder::register`] verlangt, dass `error_handler` [`RequestErrorHandler`](crate::RequestErrorHandler) für den Request-Typ ist
    /// und der Request [`Clone`] ist, denn der Handler verbraucht ihn.
    #[must_use]
    pub fn open_request_error_handler<B: Send + Sync + 'static>(
        self,
        error_handler: B,
    ) -> MediatorBuilder<S, (P, OpenBehavior<RequestErrorProcessorBehavior<B>>)> {
        self.open(
            RequestErrorProcessorBehavior(error_handler),
            Stage::ErrorHandler,
        )
    }

    /// Registriert `error_handler` nur für Requests vom Typ `R`; er bekommt jeden Fehler, bevor eine Fehler-Aktion ihn sieht.
    #[must_use]
    pub fn request_error_handler<R: Request + Clone>(
        self,
        error_handler: impl RequestErrorHandler<R> + 'static,
    ) -> MediatorBuilder<S, (P, ClosedBehavior)> {
        self.closed::<R, _>(
            RequestErrorProcessorBehavior(error_handler),
            Stage::ErrorHandler,
        )
    }

    /// Registriert `behavior` für alle Stream-Request-Typen; es liegt innerhalb der zuvor registrierten Stream-Behaviors.
    ///
    /// [`MediatorBuilder::register_stream`] verlangt, dass `behavior` [`StreamPipelineBehavior`](crate::StreamPipelineBehavior) für den Stream-Request-Typ ist.
    /// Ist ein Stream-Behavior vom Typ `B` schon registriert, läuft nur das erste, an seinem Platz.
    #[must_use]
    pub fn open_stream_behavior<B: Send + Sync + 'static>(
        self,
        behavior: B,
    ) -> MediatorBuilder<S, (P, OpenStreamBehavior<B>)> {
        self.with_behavior::<B, _>(|slot| OpenStreamBehavior {
            behavior: Arc::new(behavior),
            slot,
        })
    }

    /// Registriert `behavior` nur für Stream-Requests vom Typ `R`; es liegt innerhalb der zuvor registrierten Stream-Behaviors.
    #[must_use]
    pub fn stream_behavior<R: StreamRequest>(
        self,
        behavior: impl StreamPipelineBehavior<R> + 'static,
    ) -> MediatorBuilder<S, (P, ClosedStreamBehavior)> {
        self.closed_stream::<R, _>(behavior)
    }

    /// Builder, dessen Kette zusätzlich mit dem Stream-Behavior `behavior` vom Typ `B` nur für Stream-Requests vom Typ `R` endet.
    // Eigene Funktion, weil `stream_behavior` den Behavior-Typ nicht beim Namen kennt; hier heißt er `B`.
    fn closed_stream<R, B>(self, behavior: B) -> MediatorBuilder<S, (P, ClosedStreamBehavior)>
    where
        R: StreamRequest,
        B: StreamPipelineBehavior<R> + 'static,
    {
        self.with_behavior::<(R, B), _>(|slot| {
            let wrapper: StreamBehaviorWrapper<R> = wrap_stream_behavior(Arc::new(behavior));
            ClosedStreamBehavior {
                provider: Provider::builder()
                    .register(Registration::instance(wrapper))
                    .build(),
                slot,
            }
        })
    }

    /// Builder, dessen Kette zusätzlich mit `behavior` für alle Request-Typen auf der Stufe `stage` endet.
    fn open<B: Send + Sync + 'static>(
        self,
        behavior: B,
        stage: Stage,
    ) -> MediatorBuilder<S, (P, OpenBehavior<B>)> {
        self.with_behavior::<B, _>(|slot| OpenBehavior {
            behavior: Arc::new(behavior),
            slot,
            stage,
        })
    }

    /// Builder, dessen Kette zusätzlich mit `behavior` vom Typ `B` nur für Requests vom Typ `R` auf der Stufe `stage` endet.
    // `B` ausgeschrieben, weil die öffentlichen Methoden den Behavior-Typ nicht beim Namen kennen; der Schlüsseltyp braucht ihn.
    fn closed<R, B>(self, behavior: B, stage: Stage) -> MediatorBuilder<S, (P, ClosedBehavior)>
    where
        R: Request,
        B: PipelineBehavior<R> + 'static,
    {
        self.with_behavior::<(R, B), _>(|slot| {
            let wrapper: BehaviorWrapper<R> = wrap_behavior(Arc::new(behavior));
            ClosedBehavior {
                provider: Provider::builder()
                    .register(Registration::instance(wrapper))
                    .build(),
                slot,
                stage,
            }
        })
    }

    /// Builder, dessen Kette der Behaviors zusätzlich mit dem Glied endet, das `link` für den nächsten freien Platz baut.
    /// Der Platz läuft nur, wenn noch kein Behavior mit dem Schlüsseltyp `K` registriert ist; registriert wird er immer, einmalig je `K`.
    fn with_behavior<K: 'static, L>(
        self,
        link: impl FnOnce(Slot) -> L,
    ) -> MediatorBuilder<S, (P, L)> {
        let slot = Slot(self.slots);
        let mut active = self.active;
        if !self.provider.has_unique::<Slot, K>() {
            active.push(slot);
        }
        MediatorBuilder {
            provider: self
                .provider
                .register(Registration::instance(slot).unique_by::<K>()),
            behaviors: (self.behaviors, link(slot)),
            slots: self.slots.saturating_add(1),
            active,
            types: PhantomData,
        }
    }
}

impl<S, P, F> MediatorBuilder<S, P, F> {
    /// Registriert `registration` als Dienst; die Bauanleitung eines [`Handler::scoped_from`] holt ihn aus dem Provider.
    #[must_use]
    pub fn service<T: Send + Sync + 'static>(self, registration: Registration<T>) -> Self {
        Self {
            provider: self.provider.register(registration),
            ..self
        }
    }

    /// Registriert `handler` für Requests vom Typ `R` und baut die Pipeline der Behaviors um ihn; ein früherer Handler für `R` fällt weg.
    ///
    /// Verlangt, dass jedes Behavior für alle Request-Typen den Typ `R` kann, zum Beispiel [`PipelineBehavior`] für `R` ist.
    #[must_use]
    pub fn register<R, H>(self, handler: Handler<H, S>) -> MediatorBuilder<S, P, WithHandlers>
    where
        R: Request,
        H: RequestHandler<R> + 'static,
        P: Behaviors<R>,
    {
        let pipeline = order::behaviors(self.behaviors.wrap(&self.active));
        self.with_registration(Registration::instance(pipeline))
            .with_registration(handler.registration.map(wrap_request_handler))
    }

    /// Registriert `handler` für Stream-Requests vom Typ `R` und baut die Stream-Pipeline um ihn; ein früherer Stream-Handler für `R` fällt weg.
    ///
    /// Verlangt, dass jedes Stream-Behavior für alle Stream-Request-Typen den Typ `R` kann, also [`StreamPipelineBehavior`] für `R` ist.
    #[must_use]
    pub fn register_stream<R, H>(
        self,
        handler: Handler<H, S>,
    ) -> MediatorBuilder<S, P, WithHandlers>
    where
        R: StreamRequest,
        H: StreamRequestHandler<R> + 'static,
        P: StreamBehaviors<R>,
    {
        let pipeline = order::stream_behaviors(self.behaviors.wrap(&self.active));
        self.with_registration(Registration::instance(pipeline))
            .with_registration(handler.registration.map(wrap_stream_request_handler))
    }

    /// Registriert `handler` für Notifications vom Typ `N`; ist für `N` schon ein Handler vom Typ `H` registriert, fällt `handler` weg.
    #[must_use]
    pub fn subscribe<N, H>(self, handler: Handler<H, S>) -> MediatorBuilder<S, P, WithHandlers>
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

    /// Baut den Mediator mit allen registrierten Handlern und ihren Pipelines.
    #[must_use]
    pub fn build(self) -> Mediator<S> {
        Mediator::new(self.provider.build())
    }

    /// Builder, dessen Provider zusätzlich `registration` kennt.
    fn with_registration<T: Send + Sync + 'static>(
        self,
        registration: Registration<T>,
    ) -> MediatorBuilder<S, P, WithHandlers> {
        MediatorBuilder {
            provider: self.provider.register(registration),
            behaviors: self.behaviors,
            slots: self.slots,
            active: self.active,
            types: PhantomData,
        }
    }
}
