//! Der Vertrag, über den ein Handler Requests weiterschickt.

use std::future::Future;

use crate::Request;

/// Schickt Requests vom Typ `R` an ihren Handler; ein Handler bekommt einen Sender per Konstruktor.
///
/// [`Mediator`](crate::Mediator) schickt jeden Request-Typ. Im Test ersetzt eine Attrappe den Mediator.
///
/// # Vertrag
///
/// `send` liefert für jeden Request genau eine Antwort oder einen Fehler und löst keine Panic aus.
/// Der Sender ist `Send + Sync` und seine Future `Send`: So beweist auch ein Handler mit generischem Sender,
/// dass seine Future `Send` ist, ohne selbst `Sync` für den Sender zu verlangen.
pub trait Sender<R: Request>: Send + Sync {
    /// Schickt `request` an seinen Handler und liefert dessen Antwort.
    ///
    /// # Errors
    ///
    /// Den Fehler des Handlers, oder einen [`SendError`](crate::SendError) als `R::Error`,
    /// wenn `send` den Request nicht zustellen kann.
    fn send(&self, request: R) -> impl Future<Output = Result<R::Response, R::Error>> + Send;
}
