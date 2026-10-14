//! Der Mediator stellt Nachrichten ihren Handlern zu.

use std::error::Error;
use std::fmt;
use std::future::Future;
use std::marker::PhantomData;

use mediatr_provider::{Provider, ProviderBuilder, Registration, WeakProvider};

use crate::erased::{ErasedHandler, ErasedNotificationHandler, erase, erase_notification_handler};
use crate::pipeline::Behaviors;
use crate::request::HandlerResult;
use crate::{
    Next, Notification, NotificationHandler, PublishStrategy, Request, RequestHandler, Sequential,
};

/// Stellt Requests und Notifications den Handlern zu, die für ihren Typ registriert sind.
///
/// Entsteht über [`Mediator::builder`]. Verschickt wird über [`Sender`] und [`Publisher`].
/// `S` ist die [`PublishStrategy`], nach der `publish` die Handler laufen lässt.
/// `P` ist das Tupel der Behaviors, die jeden Request umschließen; siehe [`Mediator::with_behaviors`].
pub struct Mediator<S = Sequential, P = ()> {
    provider: Provider,
    types: PhantomData<(S, P)>,
}

/// Verweis auf einen [`Mediator`], der dessen Handler nicht am Leben hält; ein [`Handler::singleton`] bekommt ihn als [`Sender`] und [`Publisher`].
/// `S` und `P` sind Publish-Strategie und Behaviors dieses Mediators.
pub struct WeakMediator<S = Sequential, P = ()> {
    provider: WeakProvider,
    types: PhantomData<(S, P)>,
}

/// Wie der Mediator einen Handler vom Typ `H` bekommt; [`MediatorBuilder::register`] und [`MediatorBuilder::subscribe`] nehmen ihn.
/// `S` und `P` sind Publish-Strategie und Behaviors des Mediators, für den der Handler bestimmt ist.
pub struct Handler<H, S = Sequential, P = ()> {
    registration: Registration<H>,
    types: PhantomData<(S, P)>,
}

/// Sammelt Handler und baut daraus einen [`Mediator`] mit der Publish-Strategie `S` und den Behaviors `P`.
pub struct MediatorBuilder<S = Sequential, P = ()> {
    provider: ProviderBuilder,
    types: PhantomData<(S, P)>,
}

/// Schickt Requests vom Typ `R` an ihren Handler; ein Handler bekommt einen Sender per Konstruktor.
///
/// [`Mediator`] schickt jeden Request-Typ. Im Test ersetzt eine Attrappe den Mediator.
///
/// # Vertrag
///
/// `send` liefert für jeden Request genau eine Antwort oder einen Fehler und löst keine Panic aus.
pub trait Sender<R: Request> {
    /// Schickt `request` an seinen Handler und liefert dessen Antwort.
    ///
    /// # Errors
    ///
    /// Den Fehler des Handlers, oder einen [`SendError`] als `R::Error`,
    /// wenn `send` den Request nicht zustellen kann.
    fn send(&self, request: R) -> impl Future<Output = Result<R::Response, R::Error>>;
}

/// Meldet Notifications vom Typ `N` an alle ihre Handler; ein Handler bekommt einen Publisher per Konstruktor.
///
/// # Vertrag
///
/// `publish` liefert `Ok(())` oder einen Fehler und löst keine Panic aus.
pub trait Publisher<N: Notification> {
    /// Meldet `notification` an alle ihre Handler.
    ///
    /// # Errors
    ///
    /// Den Fehler eines Handlers, wie die [`PublishStrategy`] des Mediators ihn liefert;
    /// bei [`Sequential`] ist es der erste, und die Handler danach laufen nicht mehr.
    /// Oder einen [`SendError`] als `N::Error`, wenn `publish` die Notification nicht zustellen kann.
    fn publish(&self, notification: N) -> impl Future<Output = Result<(), N::Error>>;
}

/// Warum [`Sender::send`] oder [`Publisher::publish`] eine Nachricht nicht zustellen kann; beide liefern ihn als Fehlertyp der Nachricht.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SendError {
    /// Für den Request-Typ ist kein Handler registriert.
    NoHandler,
    /// Der Mediator, über den ein [`WeakMediator`] sendet oder meldet, ist weggefallen.
    MediatorDropped,
}

impl Mediator {
    /// Beginnt einen Mediator ohne Handler.
    #[must_use]
    pub fn builder() -> MediatorBuilder {
        MediatorBuilder::default()
    }

    /// Beginnt einen Mediator ohne Handler, dessen `behaviors` jeden Request umschließen; das erste liegt außen.
    ///
    /// `behaviors` ist ein Tupel mit bis zu zwei Behaviors, z. B. `(AuditTrail, Stopwatch)`.
    /// [`Sender::send`] verlangt, dass jedes davon [`PipelineBehavior`](crate::PipelineBehavior) für den Request-Typ ist.
    #[must_use]
    pub fn with_behaviors<P: 'static>(behaviors: P) -> MediatorBuilder<Sequential, P> {
        MediatorBuilder::new(behaviors)
    }
}

impl<S, P> Mediator<S, P> {
    /// Mediator, der über `provider` an Handler und Behaviors kommt.
    fn new(provider: Provider) -> Self {
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

// Von Hand statt `derive`, aus demselben Grund wie bei `Mediator`.
impl<S, P> fmt::Debug for MediatorBuilder<S, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MediatorBuilder")
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
            .get::<ErasedHandler<R>>()
            .ok_or(SendError::NoHandler)?;
        let behaviors = self
            .provider
            .get::<P>()
            .map_or_else(Vec::new, |behaviors| P::erase(&behaviors));
        Next::new(handler, behaviors).run(request).await
    }
}

impl<S: PublishStrategy, P, N: Notification + 'static> Publisher<N> for Mediator<S, P> {
    async fn publish(&self, notification: N) -> Result<(), N::Error> {
        let handlers = self.provider.all::<ErasedNotificationHandler<N>>();
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

impl<S: PublishStrategy, P, N: Notification + 'static> Publisher<N> for WeakMediator<S, P> {
    async fn publish(&self, notification: N) -> Result<(), N::Error> {
        let mediator: Mediator<S, P> =
            Mediator::new(self.provider.upgrade().ok_or(SendError::MediatorDropped)?);
        mediator.publish(notification).await
    }
}

impl<H: 'static, S: 'static, P: 'static> Handler<H, S, P> {
    /// Handler `handler`; jede Nachricht erreicht dasselbe Objekt.
    #[must_use]
    pub fn instance(handler: H) -> Self {
        Self::new(Registration::instance(handler))
    }

    /// Handler aus `make`; jede Nachricht baut ihn damit neu und gibt ihm den [`Mediator`] als [`Sender`] und [`Publisher`].
    #[must_use]
    pub fn transient(make: impl Fn(Mediator<S, P>) -> H + 'static) -> Self {
        Self::new(Registration::transient(move |provider| {
            make(Mediator::new(provider.clone()))
        }))
    }

    /// Handler aus `make`; die erste Nachricht baut ihn damit, jede weitere erreicht denselben Handler.
    ///
    /// Der Handler bekommt einen [`WeakMediator`] als [`Sender`] und [`Publisher`], damit er den Mediator nicht selbst am Leben hält.
    #[must_use]
    pub fn singleton(make: impl Fn(WeakMediator<S, P>) -> H + 'static) -> Self {
        Self::new(Registration::singleton(move |provider| {
            make(WeakMediator {
                provider: provider.downgrade(),
                types: PhantomData,
            })
        }))
    }

    /// Handler, den `registration` baut.
    fn new(registration: Registration<H>) -> Self {
        Self {
            registration,
            types: PhantomData,
        }
    }
}

impl<H, S, P> fmt::Debug for Handler<H, S, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handler")
            .field("registration", &self.registration)
            .finish()
    }
}

// Von Hand statt `derive`, aus demselben Grund wie bei `Mediator`.
impl<S, P> Clone for MediatorBuilder<S, P> {
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            types: PhantomData,
        }
    }
}

impl<S> Default for MediatorBuilder<S> {
    fn default() -> Self {
        Self::new(())
    }
}

impl<S, P: 'static> MediatorBuilder<S, P> {
    /// Builder ohne Handler, dessen Mediator jeden Request durch `behaviors` schickt.
    fn new(behaviors: P) -> Self {
        Self {
            provider: ProviderBuilder::default().register(Registration::instance(behaviors)),
            types: PhantomData,
        }
    }
}

impl<S, P> MediatorBuilder<S, P> {
    /// Registriert `handler` für Requests vom Typ `R`; ein früherer Handler für `R` fällt weg.
    #[must_use]
    pub fn register<R, H>(self, handler: Handler<H, S, P>) -> Self
    where
        R: Request + 'static,
        H: RequestHandler<R> + 'static,
    {
        self.with_registration(handler.registration.map(erase))
    }

    /// Registriert `handler` für Notifications vom Typ `N`; ist für `N` schon ein Handler vom Typ `H` registriert, fällt `handler` weg.
    #[must_use]
    pub fn subscribe<N, H>(self, handler: Handler<H, S, P>) -> Self
    where
        N: Notification + 'static,
        H: NotificationHandler<N> + 'static,
    {
        self.with_registration(
            handler
                .registration
                .map(erase_notification_handler)
                .unique_by::<H>(),
        )
    }

    /// Baut den Mediator mit allen registrierten Handlern.
    #[must_use]
    pub fn build(self) -> Mediator<S, P> {
        Mediator::new(self.provider.build())
    }

    /// Builder, dessen Provider zusätzlich `registration` kennt.
    fn with_registration<T: 'static>(self, registration: Registration<T>) -> Self {
        Self {
            provider: self.provider.register(registration),
            ..self
        }
    }
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHandler => f.write_str("kein Handler für diesen Request-Typ registriert"),
            Self::MediatorDropped => f.write_str("der Mediator ist weggefallen"),
        }
    }
}

impl Error for SendError {}
