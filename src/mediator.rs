//! Der Mediator leitet einen Request an seinen Handler weiter.

use std::error::Error;
use std::fmt;
use std::future::Future;

use mediatr_provider::{Provider, ProviderBuilder};

use crate::erased::{ErasedHandler, erase};
use crate::request::HandlerResult;
use crate::{Request, RequestHandler};

/// Leitet Requests an den Handler weiter, der für ihren Typ registriert ist.
///
/// Entsteht über [`Mediator::builder`]. Verschickt wird über [`Sender`].
#[derive(Debug, Clone)]
pub struct Mediator {
    provider: Provider,
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
    /// Den Fehler des Handlers, oder [`MissingHandler`] als `R::Error`,
    /// wenn für `R` kein Handler registriert ist.
    fn send(&self, request: R) -> impl Future<Output = Result<R::Response, R::Error>>;
}

/// Für den Request-Typ ist kein Handler registriert; [`Sender::send`] liefert ihn als `R::Error`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MissingHandler;

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
            .ok_or(MissingHandler)?;
        handler(request).await
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
            provider: self.provider.instance(erase(handler)),
        }
    }

    /// Registriert `make` für Requests vom Typ `R`; jedes `send` baut den Handler damit neu und gibt ihm den [`Mediator`] als [`Sender`].
    #[must_use]
    pub fn transient<R, H>(self, make: impl Fn(Mediator) -> H + 'static) -> Self
    where
        R: Request + 'static,
        H: RequestHandler<R> + 'static,
    {
        Self {
            provider: self.provider.transient(move |provider| {
                erase(make(Mediator {
                    provider: provider.clone(),
                }))
            }),
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

impl fmt::Display for MissingHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("kein Handler für diesen Request-Typ registriert")
    }
}

impl Error for MissingHandler {}
