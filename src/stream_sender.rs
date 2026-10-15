//! Der Vertrag, über den ein Handler Stream-Requests weiterschickt.

use futures_core::Stream;

use crate::StreamRequest;
use crate::stream_request::StreamResult;

/// Schickt Stream-Requests vom Typ `R` an ihren Handler.
///
/// # Vertrag
///
/// `create_stream` liefert für jeden Request genau einen Stream und löst keine Panic aus.
pub trait StreamSender<R: StreamRequest> {
    /// Schickt `request` an seinen Handler und liefert dessen Stream.
    ///
    /// Kann der Stream den Request nicht zustellen, liefert er einen [`SendError`](crate::SendError) als `R::Error`.
    fn create_stream(&self, request: R) -> impl Stream<Item = StreamResult<R>>;
}
