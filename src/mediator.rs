//! Der Mediator stellt Nachrichten ihren Handlern zu.

use std::fmt;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

use mediatr_provider::{Provider, WeakProvider};

use crate::pipeline::{Behaviors, Slot, StreamBehaviors, order};
use crate::request::HandlerResult;
use crate::stream_request::StreamResult;
use crate::wrappers::{
    LocalBoxStream, NotificationHandlerWrapper, RequestHandlerWrapper, StreamRequestHandlerWrapper,
};
use crate::{
    ForeachAwaitPublisher, MediatorBuilder, Next, Notification, NotificationPublisher, Publisher,
    Request, SendError, Sender, StreamNext, StreamRequest, StreamSender,
};

/// Stellt Requests und Notifications den Handlern zu, die für ihren Typ registriert sind.
///
/// Entsteht über [`Mediator::builder`]. Verschickt wird über [`Sender`] und [`Publisher`].
/// `S` ist der [`NotificationPublisher`], nach dem `publish` die Handler laufen lässt.
/// `P` ist die Kette der Behaviors, die Requests umschließen; siehe [`MediatorBuilder::open_behavior`].
pub struct Mediator<S = ForeachAwaitPublisher, P = ()> {
    provider: Provider,
    types: PhantomData<(S, P)>,
}

/// Verweis auf einen [`Mediator`], der dessen Handler nicht am Leben hält; ein [`Handler::singleton`](crate::Handler::singleton) bekommt ihn als [`Sender`] und [`Publisher`].
/// `S` und `P` sind Notification-Publisher und Behaviors dieses Mediators.
pub struct WeakMediator<S = ForeachAwaitPublisher, P = ()> {
    pub(crate) provider: WeakProvider,
    pub(crate) types: PhantomData<(S, P)>,
}

impl Mediator {
    /// Beginnt einen Mediator ohne Handler.
    #[must_use]
    pub fn builder() -> MediatorBuilder {
        MediatorBuilder::default()
    }
}

impl<S, P> Mediator<S, P> {
    /// Mediator, der über `provider` an Handler und Behaviors kommt.
    pub(crate) fn new(provider: Provider) -> Self {
        Self {
            provider,
            types: PhantomData,
        }
    }
}

// Von Hand statt `derive`: `derive(Debug)` verlangte `S: Debug` und `P: Debug`; `PhantomData` nennt beide Typen auch ohne.
impl<S, P> fmt::Debug for Mediator<S, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mediator")
            .field("provider", &self.provider)
            .field("types", &self.types)
            .finish()
    }
}

// Von Hand statt `derive`, aus demselben Grund wie bei `Mediator`.
impl<S, P> fmt::Debug for WeakMediator<S, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WeakMediator")
            .field("provider", &self.provider)
            .field("types", &self.types)
            .finish()
    }
}

// Von Hand statt `derive`: `derive(Clone)` verlangte `S: Clone` und `P: Clone`, obwohl nur `PhantomData` von ihnen abhängt.
impl<S, P> Clone for Mediator<S, P> {
    fn clone(&self) -> Self {
        Self::new(self.provider.clone())
    }
}

// Von Hand statt `derive`, aus demselben Grund wie bei `Mediator`.
impl<S, P> Clone for WeakMediator<S, P> {
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            types: PhantomData,
        }
    }
}

impl<S, P: Behaviors<R>, R: Request + 'static> Sender<R> for Mediator<S, P> {
    async fn send(&self, request: R) -> HandlerResult<R> {
        let handler = self
            .provider
            .get::<RequestHandlerWrapper<R>>()
            // Über den Mediator kommt nur `NoService` an.
            .map_err(|_| SendError::NoHandler)?;
        let active = self.provider.get::<Vec<Slot>>().unwrap_or_default();
        let behaviors = self.provider.get::<P>().map_or_else(
            |_| Vec::new(),
            |behaviors| Behaviors::<R>::wrap(&*behaviors, &active),
        );
        Next::new(handler, order::behaviors(behaviors))
            .run(request)
            .await
    }
}

impl<S: NotificationPublisher, P, N: Notification + 'static> Publisher<N> for Mediator<S, P> {
    async fn publish(&self, notification: N) -> Result<(), N::Error> {
        // Kreis und Wurzel-Fehler sind über den Mediator nicht erreichbar.
        let handlers = self
            .provider
            .all::<NotificationHandlerWrapper<N>>()
            .unwrap_or_default();
        S::publish(
            handlers
                .iter()
                .map(|handler| handler(&notification))
                .collect(),
        )
        .await
    }
}

impl<S, P: Behaviors<R>, R: Request + 'static> Sender<R> for WeakMediator<S, P> {
    async fn send(&self, request: R) -> HandlerResult<R> {
        let mediator: Mediator<S, P> =
            Mediator::new(self.provider.upgrade().ok_or(SendError::MediatorDropped)?);
        mediator.send(request).await
    }
}

impl<S: NotificationPublisher, P, N: Notification + 'static> Publisher<N> for WeakMediator<S, P> {
    async fn publish(&self, notification: N) -> Result<(), N::Error> {
        let mediator: Mediator<S, P> =
            Mediator::new(self.provider.upgrade().ok_or(SendError::MediatorDropped)?);
        mediator.publish(notification).await
    }
}

impl<S, P: StreamBehaviors<R>, R: StreamRequest + 'static> StreamSender<R> for Mediator<S, P> {
    fn create_stream(&self, request: R) -> impl Stream<Item = StreamResult<R>> {
        let active = self.provider.get::<Vec<Slot>>().unwrap_or_default();
        let behaviors = self.provider.get::<P>().map_or_else(
            |_| Vec::new(),
            |behaviors| StreamBehaviors::<R>::wrap(&*behaviors, &active),
        );
        // dyn: der Stream der Pipeline und `Once` sind verschiedene Typen und brauchen einen gemeinsamen.
        let stream: LocalBoxStream<StreamResult<R>> =
            match self.provider.get::<StreamRequestHandlerWrapper<R>>() {
                Ok(handler) => Box::pin(
                    StreamNext::new(handler, order::stream_behaviors(behaviors)).run(request),
                ),
                // Über den Mediator kommt nur `NoService` an.
                Err(_) => Box::pin(Once(Some(Err(R::Error::from(SendError::NoHandler))))),
            };
        stream
    }
}

/// Ein Stream, der höchstens ein Element liefert und danach endet.
struct Once<T>(Option<T>);

// Von Hand: `Once` hält sein Element nie angeheftet, es nimmt es nur heraus.
impl<T> Unpin for Once<T> {}

impl<T> Stream for Once<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<T>> {
        Poll::Ready(self.get_mut().0.take())
    }
}
