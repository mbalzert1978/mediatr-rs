//! Verträge für Requests, die genau ein Handler beantwortet.

use std::future::Future;

/// Eine Nachricht, die genau ein Handler mit einer Antwort beantwortet.
///
/// # Vertrag
///
/// `Response` ist der einzige Typ, den ein Handler für diesen Request liefert.
pub trait Request {
    /// Typ der Antwort, die `send` für diesen Request liefert.
    type Response;
}

/// Beantwortet Requests vom Typ `R`.
///
/// # Vertrag
///
/// `handle` liefert für jeden Request genau eine Antwort und löst keine Panic aus.
pub trait RequestHandler<R: Request> {
    /// Beantwortet `request`.
    fn handle(&self, request: R) -> impl Future<Output = R::Response>;
}
