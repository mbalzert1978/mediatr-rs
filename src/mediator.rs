//! Der Mediator leitet einen Request an seinen Handler weiter.

use std::error::Error;
use std::fmt;

use crate::registry::Registry;
use crate::request::HandlerResult;
use crate::{Request, RequestHandler};

/// Leitet Requests an den Handler weiter, der für ihren Typ registriert ist.
///
/// Entsteht über [`Mediator::builder`].
#[derive(Debug)]
pub struct Mediator {
    registry: Registry,
}

/// Sammelt Handler und baut daraus einen [`Mediator`].
#[derive(Debug, Default)]
pub struct MediatorBuilder {
    registry: Registry,
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
        let handler = self.registry.handler::<R>().ok_or(MissingHandler)?;
        handler(request).await
    }
}

impl MediatorBuilder {
    /// Registriert `handler` für Requests vom Typ `R`; ein früherer Handler für `R` fällt weg.
    #[must_use]
    pub fn register<R, H>(mut self, handler: H) -> Self
    where
        R: Request + 'static,
        H: RequestHandler<R> + 'static,
    {
        self.registry.register(handler);
        self
    }

    /// Baut den Mediator mit allen registrierten Handlern.
    #[must_use]
    pub fn build(self) -> Mediator {
        Mediator {
            registry: self.registry,
        }
    }
}

impl fmt::Display for MissingHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("kein Handler für diesen Request-Typ registriert")
    }
}

impl Error for MissingHandler {}
