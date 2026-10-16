//! Der Mediator stellt Nachrichten ihren Handlern zu.

use std::fmt;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

use mediatr_provider::{GetError, Provider, WeakProvider};

use crate::pipeline::{Pipeline, StreamPipeline};
use crate::request::HandlerResult;
use crate::stream_request::StreamResult;
use crate::wrappers::{
    BoxStream, NotificationHandlerWrapper, RequestHandlerWrapper, StreamRequestHandlerWrapper,
};
use crate::{
    ForeachAwaitPublisher, MediatorBuilder, Next, Notification, NotificationPublisher, Publisher,
    Request, SendError, Sender, StreamNext, StreamRequest, StreamSender,
};

/// Stellt Requests und Notifications den Handlern zu, die für ihren Typ registriert sind.
///
/// Entsteht über [`Mediator::builder`]. Verschickt wird über [`Sender`] und [`Publisher`].
/// `S` ist der [`NotificationPublisher`], nach dem `publish` die Handler laufen lässt.
/// Die Behaviors, die Requests umschließen, baut [`MediatorBuilder::register`] je Request-Typ in die Pipeline; der Typ nennt sie nicht.
pub struct Mediator<S = ForeachAwaitPublisher> {
    provider: Provider,
    publisher: PhantomData<S>,
}

/// Verweis auf einen [`Mediator`], der dessen Handler nicht am Leben hält; ein [`Handler::singleton`](crate::Handler::singleton) oder [`Handler::scoped`](crate::Handler::scoped) bekommt ihn als [`Sender`] und [`Publisher`].
/// `S` ist der Notification-Publisher dieses Mediators.
pub struct WeakMediator<S = ForeachAwaitPublisher> {
    pub(crate) provider: WeakProvider,
    pub(crate) publisher: PhantomData<S>,
}

impl Mediator {
    /// Beginnt einen Mediator ohne Handler.
    #[must_use]
    pub fn builder() -> MediatorBuilder {
        MediatorBuilder::default()
    }
}

impl<S> Mediator<S> {
    /// Mediator, der über `provider` an Handler und Pipelines kommt.
    pub(crate) fn new(provider: Provider) -> Self {
        Self {
            provider,
            publisher: PhantomData,
        }
    }

    /// Beginnt einen neuen Scope mit denselben Handlern; [`Handler::scoped`](crate::Handler::scoped)-Handler baut er neu.
    #[must_use]
    pub fn scope(&self) -> Self {
        Self::new(self.provider.scope())
    }
}

// Von Hand statt `derive`: `derive(Debug)` verlangte `S: Debug`; `PhantomData` nennt den Publisher auch ohne.
impl<S> fmt::Debug for Mediator<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mediator")
            .field("provider", &self.provider)
            .field("publisher", &self.publisher)
            .finish()
    }
}

// Von Hand statt `derive`, aus demselben Grund wie bei `Mediator`.
impl<S> fmt::Debug for WeakMediator<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WeakMediator")
            .field("provider", &self.provider)
            .field("publisher", &self.publisher)
            .finish()
    }
}

// Von Hand statt `derive`: `derive(Clone)` verlangte `S: Clone`, obwohl nur `PhantomData` von `S` abhängt.
impl<S> Clone for Mediator<S> {
    fn clone(&self) -> Self {
        Self::new(self.provider.clone())
    }
}

// Von Hand statt `derive`, aus demselben Grund wie bei `Mediator`.
impl<S> Clone for WeakMediator<S> {
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            publisher: PhantomData,
        }
    }
}

/// Warum der Mediator eine Nachricht nicht zustellt, wenn der Provider ihren Handler nicht liefert.
fn undeliverable(error: &GetError) -> SendError {
    match error {
        GetError::ScopedFromRoot => SendError::NoScope,
        // Über den Mediator kommt sonst nur `NoService` an.
        _ => SendError::NoHandler,
    }
}

impl<S: NotificationPublisher, R: Request> Sender<R> for Mediator<S> {
    async fn send(&self, request: R) -> HandlerResult<R> {
        let handler = self
            .provider
            .get::<RequestHandlerWrapper<R>>()
            .map_err(|error| undeliverable(&error))?;
        // Die Pipeline fehlt nur, wenn der Handler fehlt; dann ist `send` schon raus.
        let pipeline = self
            .provider
            .get::<Pipeline<R>>()
            .map_or_else(|_| Vec::new(), |pipeline| (*pipeline).clone());
        Next::new(handler, pipeline).run(request).await
    }
}

impl<S: NotificationPublisher, N: Notification + 'static> Publisher<N> for Mediator<S> {
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

impl<S: NotificationPublisher, R: Request> Sender<R> for WeakMediator<S> {
    async fn send(&self, request: R) -> HandlerResult<R> {
        let mediator: Mediator<S> =
            Mediator::new(self.provider.upgrade().ok_or(SendError::MediatorDropped)?);
        mediator.send(request).await
    }
}

impl<S: NotificationPublisher, N: Notification + 'static> Publisher<N> for WeakMediator<S> {
    async fn publish(&self, notification: N) -> Result<(), N::Error> {
        let mediator: Mediator<S> =
            Mediator::new(self.provider.upgrade().ok_or(SendError::MediatorDropped)?);
        mediator.publish(notification).await
    }
}

impl<S: NotificationPublisher, R: StreamRequest> StreamSender<R> for Mediator<S> {
    fn create_stream(&self, request: R) -> impl Stream<Item = StreamResult<R>> + Send {
        // Die Stream-Pipeline fehlt nur, wenn der Handler fehlt; dann liefert der Stream den Fehler.
        let pipeline = self
            .provider
            .get::<StreamPipeline<R>>()
            .map_or_else(|_| Vec::new(), |pipeline| (*pipeline).clone());
        // dyn: der Stream der Pipeline und `Once` sind verschiedene Typen und brauchen einen gemeinsamen.
        let stream: BoxStream<StreamResult<R>> =
            match self.provider.get::<StreamRequestHandlerWrapper<R>>() {
                Ok(handler) => Box::pin(StreamNext::new(handler, pipeline).run(request)),
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
