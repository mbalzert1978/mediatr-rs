//! Der Mediator leitet einen Request an seinen Handler weiter.

use std::error::Error;
use std::fmt;

use crate::request::HandlerResult;
use crate::{HandlerProvider, Request, RequestHandler};

/// Leitet Requests an den Handler weiter, den der Provider `P` für ihren Typ liefert.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Mediator<P> {
    provider: P,
}

/// Baut einen [`Mediator`]; entsteht über [`Mediator::builder`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MediatorBuilder<P> {
    provider: P,
}

/// Für den Request-Typ ist kein Handler registriert; [`Mediator::send`] liefert ihn als `R::Error`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MissingHandler;

impl<P> Mediator<P> {
    /// Beginnt einen Mediator, der seine Handler von `provider` holt.
    pub fn builder(provider: P) -> MediatorBuilder<P> {
        MediatorBuilder { provider }
    }

    /// Schickt `request` an seinen Handler und liefert dessen Antwort.
    ///
    /// # Errors
    ///
    /// Den Fehler des Handlers, oder [`MissingHandler`] als `R::Error`,
    /// wenn der Provider für `R` keinen Handler liefert.
    pub async fn send<R>(&self, request: R) -> HandlerResult<R>
    where
        R: Request,
        P: HandlerProvider<R>,
    {
        let handler = self.provider.handler().ok_or(MissingHandler)?;
        handler.handle(request).await
    }
}

impl<P> MediatorBuilder<P> {
    /// Baut den Mediator.
    pub fn build(self) -> Mediator<P> {
        Mediator {
            provider: self.provider,
        }
    }
}

impl fmt::Display for MissingHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("kein Handler für diesen Request-Typ registriert")
    }
}

impl Error for MissingHandler {}
