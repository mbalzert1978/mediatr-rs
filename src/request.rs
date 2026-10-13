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
    fn handle(&self, request: R) -> impl Future<Output = Result<R::Response, R::Error>>;
}
