//! Der Mediator stellt Nachrichten ihren Handlern zu.

use std::error::Error;
use std::fmt;
use std::future::Future;

use mediatr_provider::{Provider, ProviderBuilder, Registration, WeakProvider};

use crate::erased::{ErasedHandler, ErasedNotificationHandler, erase, erase_notification_handler};
use crate::request::HandlerResult;
use crate::{Notification, NotificationHandler, Request, RequestHandler};

/// Stellt Requests und Notifications den Handlern zu, die für ihren Typ registriert sind.
///
/// Entsteht über [`Mediator::builder`]. Verschickt wird über [`Sender`] und [`Publisher`].
#[derive(Debug, Clone)]
pub struct Mediator {
    provider: Provider,
}

/// Verweis auf einen [`Mediator`], der dessen Handler nicht am Leben hält; ein `singleton`-Handler bekommt ihn als [`Sender`] und [`Publisher`].
#[derive(Debug, Clone)]
pub struct WeakMediator {
    provider: WeakProvider,
}

/// Sammelt Handler und baut daraus einen [`Mediator`].
#[derive(Debug, Clone, Default)]
pub struct MediatorBuilder {
    provider: ProviderBuilder,
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
    /// Den ersten Fehler eines Handlers; die Handler danach laufen nicht mehr.
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
}

impl<R: Request + 'static> Sender<R> for Mediator {
    async fn send(&self, request: R) -> HandlerResult<R> {
        let handler = self
            .provider
            .get::<ErasedHandler<R>>()
            .ok_or(SendError::NoHandler)?;
        handler(request).await
    }
}

impl<N: Notification + 'static> Publisher<N> for Mediator {
    async fn publish(&self, notification: N) -> Result<(), N::Error> {
        for handler in self.provider.all::<ErasedNotificationHandler<N>>() {
            handler(&notification).await?;
        }
        Ok(())
    }
}

impl<R: Request + 'static> Sender<R> for WeakMediator {
    async fn send(&self, request: R) -> HandlerResult<R> {
        let mediator = Mediator {
            provider: self.provider.upgrade().ok_or(SendError::MediatorDropped)?,
        };
        mediator.send(request).await
    }
}

impl<N: Notification + 'static> Publisher<N> for WeakMediator {
    async fn publish(&self, notification: N) -> Result<(), N::Error> {
        let mediator = Mediator {
            provider: self.provider.upgrade().ok_or(SendError::MediatorDropped)?,
        };
        mediator.publish(notification).await
    }
}

impl MediatorBuilder {
    /// Registriert `handler` für Requests vom Typ `R`; ein früherer Handler für `R` fällt weg.
    #[must_use]
    pub fn register<R, H>(self, handler: H) -> Self
    where
        R: Request + 'static,
        H: RequestHandler<R> + 'static,
    {
        Self {
            provider: self
                .provider
                .register(Registration::instance(erase(handler))),
        }
    }

    /// Registriert `make` für Requests vom Typ `R`; jedes `send` baut den Handler damit neu und gibt ihm den [`Mediator`] als [`Sender`] und [`Publisher`].
    #[must_use]
    pub fn transient<R, H>(self, make: impl Fn(Mediator) -> H + 'static) -> Self
    where
        R: Request + 'static,
        H: RequestHandler<R> + 'static,
    {
        Self {
            provider: self
                .provider
                .register(Registration::transient(move |provider| {
                    erase(make(Mediator {
                        provider: provider.clone(),
                    }))
                })),
        }
    }

    /// Registriert `make` für Requests vom Typ `R`; das erste `send` baut den Handler damit, jedes weitere nutzt denselben Handler.
    ///
    /// Der Handler bekommt einen [`WeakMediator`] als [`Sender`] und [`Publisher`], damit er den Mediator nicht selbst am Leben hält.
    #[must_use]
    pub fn singleton<R, H>(self, make: impl Fn(WeakMediator) -> H + 'static) -> Self
    where
        R: Request + 'static,
        H: RequestHandler<R> + 'static,
    {
        Self {
            provider: self
                .provider
                .register(Registration::singleton(move |provider| {
                    erase(make(WeakMediator {
                        provider: provider.downgrade(),
                    }))
                })),
        }
    }

    /// Registriert `handler` für Notifications vom Typ `N`; ist für `N` schon ein Handler vom Typ `H` registriert, fällt `handler` weg.
    #[must_use]
    pub fn subscribe<N, H>(self, handler: H) -> Self
    where
        N: Notification + 'static,
        H: NotificationHandler<N> + 'static,
    {
        Self {
            provider: self.provider.register(
                Registration::instance(erase_notification_handler(handler)).unique_by::<H>(),
            ),
        }
    }

    /// Registriert `make` für Notifications vom Typ `N`; jedes `publish` baut den Handler damit neu und gibt ihm den [`Mediator`] als [`Sender`] und [`Publisher`]. Ist für `N` schon ein Handler vom Typ `H` registriert, fällt `make` weg.
    #[must_use]
    pub fn subscribe_transient<N, H>(self, make: impl Fn(Mediator) -> H + 'static) -> Self
    where
        N: Notification + 'static,
        H: NotificationHandler<N> + 'static,
    {
        Self {
            provider: self.provider.register(
                Registration::transient(move |provider| {
                    erase_notification_handler(make(Mediator {
                        provider: provider.clone(),
                    }))
                })
                .unique_by::<H>(),
            ),
        }
    }

    /// Baut den Mediator mit allen registrierten Handlern.
    #[must_use]
    pub fn build(self) -> Mediator {
        Mediator {
            provider: self.provider.build(),
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
