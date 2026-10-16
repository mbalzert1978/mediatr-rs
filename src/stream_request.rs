//! Verträge für Stream-Requests, die genau ein Handler mit einer Folge von Elementen beantwortet.

use futures_core::Stream;

use crate::SendError;

/// Eine Nachricht, die genau ein Handler mit einer Folge von Elementen beantwortet.
///
/// # Vertrag
///
/// `Item` ist der einzige Typ, den ein Handler für diesen Request als Element liefert.
/// `Error` ist der einzige Fehlertyp, den der Stream für diesen Request liefert.
/// `From<SendError>` liefert einen Wert, der „der Stream kann den Request nicht zustellen“ bedeutet.
/// Request, Elemente und Fehler sind `Send`, und der Request ist `'static`: `create_stream` liefert einen `Send`-Stream,
/// den eine mit `tokio::spawn` gestartete Aufgabe auf einen anderen Thread tragen kann.
pub trait StreamRequest: Send + 'static {
    /// Typ der Elemente, die der Stream für diesen Request liefert.
    type Item: Send;
    /// Typ des Fehlers, den der Stream für diesen Request liefert.
    type Error: From<SendError> + Send;
}

/// Was ein Stream für `R` je Schritt liefert: ein Element oder den Fehler von `R`.
pub(crate) type StreamResult<R> = Result<<R as StreamRequest>::Item, <R as StreamRequest>::Error>;

/// Beantwortet Stream-Requests vom Typ `R` mit einer Folge von Elementen.
///
/// # Vertrag
///
/// `handle` liefert für jeden Request genau einen Stream und löst keine Panic aus; der Stream leiht nichts vom Handler.
/// Der Handler ist `Send + Sync` und sein Stream `Send`: Mehrere Threads teilen sich einen Handler.
pub trait StreamRequestHandler<R: StreamRequest>: Send + Sync {
    /// Beantwortet `request` mit einem Stream seiner Elemente.
    fn handle(&self, request: R) -> impl Stream<Item = StreamResult<R>> + Send + use<Self, R>;
}
