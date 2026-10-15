//! Die Pipeline um den Handler eines Requests.

mod error_action;
mod next;
mod post_processor;
mod pre_processor;
mod sealed;

use std::future::Future;

use crate::Request;
use crate::request::HandlerResult;

pub use error_action::RequestErrorAction;
pub use next::Next;
pub use post_processor::RequestPostProcessor;
pub use pre_processor::RequestPreProcessor;
pub(crate) use sealed::{
    Behaviors, Closed, OpenBehavior, RequestErrorActionProcessorBehavior,
    RequestPostProcessorBehavior, RequestPreProcessorBehavior, Slot, Stage,
};

/// Umschließt den Handler von Requests vom Typ `R`: läuft davor und danach und entscheidet, ob es weitergeht.
///
/// # Vertrag
///
/// `handle` liefert für jeden Request genau eine Antwort oder einen Fehler und löst keine Panic aus.
pub trait PipelineBehavior<R: Request> {
    /// Bearbeitet `request`; `next` führt zum nächsten Behavior oder zum Handler.
    ///
    /// # Errors
    ///
    /// `R::Error`, wenn das Behavior oder ein späterer Schritt `request` nicht beantworten kann.
    fn handle(&self, request: R, next: Next<R>) -> impl Future<Output = HandlerResult<R>>;
}
