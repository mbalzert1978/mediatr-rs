//! Der Mediator leitet einen Request an seinen Handler weiter.

use crate::registry::Registry;
use crate::{Request, RequestHandler};

/// Leitet Requests an den Handler weiter, der für ihren Typ registriert ist.
#[derive(Debug, Default)]
pub struct Mediator {
    registry: Registry,
}

/// Für den Request-Typ ist kein Handler registriert; [`Mediator::send`] liefert ihn als `R::Error`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MissingHandler;

impl Mediator {
    /// Registriert `handler` für Requests vom Typ `R`.
    pub fn register<R, H>(&mut self, handler: H)
    where
        R: Request + 'static,
        H: RequestHandler<R> + 'static,
    {
        self.registry.insert(handler);
    }

    /// Schickt `request` an seinen Handler und liefert dessen Antwort.
    ///
    /// # Errors
    ///
    /// Den Fehler des Handlers, oder [`MissingHandler`] als `R::Error`,
    /// wenn für `R` kein Handler registriert ist.
    pub async fn send<R: Request + 'static>(&self, request: R) -> Result<R::Response, R::Error> {
        let handler = self.registry.get::<R>().ok_or(MissingHandler)?;
        handler(request).await
    }
}
