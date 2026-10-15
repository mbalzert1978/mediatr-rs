//! Das Stream-Behavior, das den Stream eines Stream-Requests umschließt.

use futures_core::Stream;

use crate::stream_request::StreamResult;
use crate::{StreamNext, StreamRequest};

/// Umschließt den Stream-Handler von Stream-Requests vom Typ `R`: läuft davor, nach dem letzten Element oder entscheidet, ob es weitergeht.
///
/// # Vertrag
///
/// `handle` liefert für jeden Request genau einen Stream und löst keine Panic aus; der Stream leiht nichts vom Behavior.
pub trait StreamPipelineBehavior<R: StreamRequest> {
    /// Bearbeitet `request`; `next` führt zum nächsten Stream-Behavior oder zum Stream-Handler.
    fn handle(
        &self,
        request: R,
        next: StreamNext<R>,
    ) -> impl Stream<Item = StreamResult<R>> + use<Self, R>;
}
