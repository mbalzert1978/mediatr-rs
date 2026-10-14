//! Der Mediator stellt Nachrichten ihren Handlern zu.

use std::fmt;
use std::marker::PhantomData;

use mediatr_provider::{Provider, WeakProvider};

use crate::pipeline::{Behaviors, Slot};
use crate::request::HandlerResult;
use crate::wrappers::{NotificationHandlerWrapper, RequestHandlerWrapper};
use crate::{
    ForeachAwaitPublisher, MediatorBuilder, Next, Notification, NotificationPublisher, Publisher,
    Request, SendError, Sender,
};

/// Stellt Requests und Notifications den Handlern zu, die für ihren Typ registriert sind.
///
/// Entsteht über [`Mediator::builder`]. Verschickt wird über [`Sender`] und [`Publisher`].
/// `S` ist die [`NotificationPublisher`], nach der `publish` die Handler laufen lässt.
/// `P` ist die Kette der Behaviors, die Requests umschließen; siehe [`MediatorBuilder::behavior`].
pub struct Mediator<S = ForeachAwaitPublisher, P = ()> {
    provider: Provider,
    types: PhantomData<(S, P)>,
}

/// Verweis auf einen [`Mediator`], der dessen Handler nicht am Leben hält; ein [`Handler::singleton`](crate::Handler::singleton) bekommt ihn als [`Sender`] und [`Publisher`].
/// `S` und `P` sind Publish-Strategie und Behaviors dieses Mediators.
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
            .ok_or(SendError::NoHandler)?;
        // ponytail: liest die behaltenen Plätze bei jedem `send`; einmal beim Bauen merken, wenn das messbar bremst.
        let active: Vec<Slot> = self
            .provider
            .all::<Slot>()
            .iter()
            .map(|slot| **slot)
            .collect();
        let behaviors = self
            .provider
            .get::<P>()
            .map_or_else(Vec::new, |behaviors| behaviors.wrap(&active));
        Next::new(handler, &behaviors).run(request).await
    }
}

impl<S: NotificationPublisher, P, N: Notification + 'static> Publisher<N> for Mediator<S, P> {
    async fn publish(&self, notification: N) -> Result<(), N::Error> {
        let handlers = self.provider.all::<NotificationHandlerWrapper<N>>();
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
