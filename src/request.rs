//! Verträge für Requests, die genau ein Handler beantwortet.

use std::future::Future;

use crate::MissingHandler;

/// Eine Nachricht, die genau ein Handler mit einer Antwort beantwortet.
///
/// # Vertrag
///
/// `Response` ist der einzige Typ, den ein Handler für diesen Request liefert.
/// `Error` ist der einzige Fehlertyp, den `send` für diesen Request liefert.
/// `From<MissingHandler>` liefert einen Wert, der „kein Handler registriert“ bedeutet.
pub trait Request {
    /// Typ der Antwort, die `send` für diesen Request liefert.
    type Response;
    /// Typ des Fehlers, den `send` für diesen Request liefert.
    type Error: From<MissingHandler>;
}

/// Was ein Handler für `R` liefert: die Antwort oder den Fehler von `R`.
pub(crate) type HandlerResult<R> = Result<<R as Request>::Response, <R as Request>::Error>;

/// Beantwortet Requests vom Typ `R`.
///
/// # Vertrag
///
/// `handle` liefert für jeden Request genau eine Antwort oder einen Fehler und löst keine Panic aus.
pub trait RequestHandler<R: Request> {
    /// Beantwortet `request`.
    ///
    /// # Errors
    ///
    /// `R::Error`, wenn der Handler `request` nicht beantworten kann.
    fn handle(&self, request: R) -> impl Future<Output = HandlerResult<R>>;
}

/// Jede async-Funktion und jede Async-Closure von `R` nach `Result` ist ein Handler für `R`.
///
/// Der Vertrag von [`RequestHandler`] gilt auch für sie: Sie löst keine Panic aus.
impl<R, F> RequestHandler<R> for F
where
    R: Request,
    F: AsyncFn(R) -> HandlerResult<R>,
{
    fn handle(&self, request: R) -> impl Future<Output = HandlerResult<R>> {
        self(request)
    }
}
