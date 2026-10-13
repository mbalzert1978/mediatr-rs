//! Der Mediator leitet einen Request an seinen Handler weiter.

use std::error::Error;
use std::fmt;

use mediatr_provider::{Provider, ProviderBuilder};

use crate::erased::{ErasedHandler, erase};
use crate::request::HandlerResult;
use crate::{Request, RequestHandler};

/// Leitet Requests an den Handler weiter, der für ihren Typ registriert ist.
///
/// Entsteht über [`Mediator::builder`].
#[derive(Debug, Clone)]
pub struct Mediator {
    sender: Sender,
}

/// Sammelt Handler und baut daraus einen [`Mediator`].
#[derive(Debug, Clone, Default)]
pub struct MediatorBuilder {
    provider: ProviderBuilder,
}

/// Schickt Requests an ihren Handler; ein Handler bekommt ihn per Konstruktor.
#[derive(Debug, Clone)]
pub struct Sender {
    provider: Provider,
}

/// Für den Request-Typ ist kein Handler registriert; [`Mediator::send`] liefert ihn als `R::Error`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MissingHandler;

impl Mediator {
    /// Beginnt einen Mediator ohne Handler.
    #[must_use]
    pub fn builder() -> MediatorBuilder {
        MediatorBuilder::default()
    }

    /// Schickt `request` an seinen Handler und liefert dessen Antwort.
    ///
    /// # Errors
    ///
    /// Den Fehler des Handlers, oder [`MissingHandler`] als `R::Error`,
    /// wenn für `R` kein Handler registriert ist.
    pub async fn send<R: Request + 'static>(&self, request: R) -> HandlerResult<R> {
        self.sender.send(request).await
    }
}

impl Sender {
    /// Schickt `request` an seinen Handler und liefert dessen Antwort.
    ///
    /// # Errors
    ///
    /// Wie [`Mediator::send`].
    pub async fn send<R: Request + 'static>(&self, request: R) -> HandlerResult<R> {
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

    /// Registriert `make` für Requests vom Typ `R`; jedes `send` baut den Handler damit neu und gibt ihm einen [`Sender`].
    #[must_use]
    pub fn transient<R, H>(self, make: impl Fn(Sender) -> H + 'static) -> Self
    where
        R: Request + 'static,
        H: RequestHandler<R> + 'static,
    {
        Self {
            provider: self.provider.transient(move |provider| {
                erase(make(Sender {
                    provider: provider.clone(),
                }))
            }),
        }
    }

    /// Baut den Mediator mit allen registrierten Handlern.
    #[must_use]
    pub fn build(self) -> Mediator {
        Mediator {
            sender: Sender {
                provider: self.provider.build(),
            },
        }
    }
}

impl fmt::Display for MissingHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("kein Handler für diesen Request-Typ registriert")
    }
}

impl Error for MissingHandler {}
