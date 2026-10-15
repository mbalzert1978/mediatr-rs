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
pub trait StreamRequest {
    /// Typ der Elemente, die der Stream für diesen Request liefert.
    type Item;
    /// Typ des Fehlers, den der Stream für diesen Request liefert.
    type Error: From<SendError>;
}

/// Was ein Stream für `R` je Schritt liefert: ein Element oder den Fehler von `R`.
pub(crate) type StreamResult<R> = Result<<R as StreamRequest>::Item, <R as StreamRequest>::Error>;

/// Beantwortet Stream-Requests vom Typ `R` mit einer Folge von Elementen.
///
/// # Vertrag
///
/// `handle` liefert für jeden Request genau einen Stream und löst keine Panic aus; der Stream leiht nichts vom Handler.
pub trait StreamRequestHandler<R: StreamRequest> {
    /// Beantwortet `request` mit einem Stream seiner Elemente.
    fn handle(&self, request: R) -> impl Stream<Item = StreamResult<R>> + use<Self, R>;
}
