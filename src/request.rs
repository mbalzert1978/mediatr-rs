//! Verträge für Requests, die genau ein Handler beantwortet.

use std::future::Future;

use crate::SendError;

/// Eine Nachricht, die genau ein Handler mit einer Antwort beantwortet.
///
/// # Vertrag
///
/// `Response` ist der einzige Typ, den ein Handler für diesen Request liefert.
/// `Error` ist der einzige Fehlertyp, den `send` für diesen Request liefert.
/// `From<SendError>` liefert einen Wert, der „`send` kann den Request nicht zustellen“ bedeutet.
/// Request, Antwort und Fehler sind `Send`, und der Request ist `'static`: `send` liefert eine `Send`-Future,
/// die eine mit `tokio::spawn` gestartete Aufgabe auf einen anderen Thread tragen kann.
/// Request, Antwort und Fehler sind auch `Sync`: Vor- und Nachverarbeiter und Fehler-Aktionen leihen sie über ein `await`.
pub trait Request: Send + Sync + 'static {
    /// Typ der Antwort, die `send` für diesen Request liefert.
    type Response: Send + Sync;
    /// Typ des Fehlers, den `send` für diesen Request liefert.
    type Error: From<SendError> + Send + Sync;
}

/// Was ein Handler für `R` liefert: die Antwort oder den Fehler von `R`.
pub(crate) type HandlerResult<R> = Result<<R as Request>::Response, <R as Request>::Error>;

/// Beantwortet Requests vom Typ `R`.
///
/// # Vertrag
///
/// `handle` liefert für jeden Request genau eine Antwort oder einen Fehler und löst keine Panic aus.
/// Der Handler ist `Send + Sync` und seine Future `Send`: Mehrere Threads teilen sich einen Handler.
pub trait RequestHandler<R: Request>: Send + Sync {
    /// Beantwortet `request`.
    ///
    /// # Errors
    ///
    /// `R::Error`, wenn der Handler `request` nicht beantworten kann.
    fn handle(&self, request: R) -> impl Future<Output = HandlerResult<R>> + Send;
}

/// Jede async-Funktion von `R` nach `Result` ist ein Handler für `R`, ebenso `async |request| { … }` ohne `move`
/// und `|request| async move { … }`; Closure und Future müssen dafür `Send` sein.
///
/// `async move |request| { … }` mit Zustand ist kein Handler: Aus `AsyncFn` lässt sich kein `Send` für die Future verlangen.
///
/// Der Vertrag von [`RequestHandler`] gilt auch für sie: Sie löst keine Panic aus.
impl<R, F, Fut> RequestHandler<R> for F
where
    R: Request,
    F: Fn(R) -> Fut + Send + Sync,
    Fut: Future<Output = HandlerResult<R>> + Send,
{
    fn handle(&self, request: R) -> impl Future<Output = HandlerResult<R>> + Send {
        self(request)
    }
}
