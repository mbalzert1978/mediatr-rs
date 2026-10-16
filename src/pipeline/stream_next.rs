//! Der Rest der Stream-Pipeline hinter einem Stream-Behavior.

use std::fmt;
use std::sync::Arc;

use futures_core::Stream;

use crate::StreamRequest;
use crate::pipeline::Deferred;
use crate::stream_request::StreamResult;
use crate::wrappers::{StreamBehaviorWrapper, StreamRequestHandlerWrapper};

/// Die Stream-Behaviors einer Stream-Pipeline für Stream-Requests vom Typ `R`, geordnet, das innerste zuerst; [`MediatorBuilder::register_stream`](crate::MediatorBuilder::register_stream) baut sie einmal je Typ.
pub(crate) type StreamPipeline<R> = Vec<Arc<StreamBehaviorWrapper<R>>>;

/// Der Rest der Stream-Pipeline hinter einem Stream-Behavior; [`StreamNext::run`] schickt einen Stream-Request hinein.
pub struct StreamNext<R: StreamRequest> {
    handler: Arc<StreamRequestHandlerWrapper<R>>,
    /// Die noch ausstehenden Stream-Behaviors, das innerste zuerst; `run` nimmt das letzte, also äußerste.
    behaviors: StreamPipeline<R>,
}

impl<R: StreamRequest> StreamNext<R> {
    /// Stream-Pipeline aus `behaviors`, geordnet wie [`order::stream_behaviors`](crate::pipeline::order::stream_behaviors) sie liefert, und `handler`.
    pub(crate) fn new(
        handler: Arc<StreamRequestHandlerWrapper<R>>,
        behaviors: StreamPipeline<R>,
    ) -> Self {
        Self { handler, behaviors }
    }

    /// Schickt `request` an den nächsten Schritt und liefert dessen Stream; der nächste Schritt läuft erst, wenn jemand den Stream liest.
    pub fn run(mut self, request: R) -> impl Stream<Item = StreamResult<R>> + use<R> {
        Deferred::new(move || match self.behaviors.pop() {
            Some(behavior) => behavior(request, self),
            None => (self.handler)(request),
        })
    }
}

// Von Hand statt `derive`: `derive(Clone)` verlangte `R: Clone`, obwohl nur die `Arc`-Felder geklont werden.
impl<R: StreamRequest> Clone for StreamNext<R> {
    fn clone(&self) -> Self {
        Self {
            handler: Arc::clone(&self.handler),
            behaviors: self.behaviors.clone(),
        }
    }
}

impl<R: StreamRequest> fmt::Debug for StreamNext<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamNext").finish_non_exhaustive()
    }
}
