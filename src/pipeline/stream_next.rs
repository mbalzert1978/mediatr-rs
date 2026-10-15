//! Der Rest der Stream-Pipeline hinter einem Stream-Behavior.

use std::fmt;
use std::rc::Rc;

use futures_core::Stream;

use crate::StreamRequest;
use crate::pipeline::Deferred;
use crate::stream_request::StreamResult;
use crate::wrappers::{StreamBehaviorWrapper, StreamRequestHandlerWrapper};

/// Der Rest der Stream-Pipeline hinter einem Stream-Behavior; [`StreamNext::run`] schickt einen Stream-Request hinein.
pub struct StreamNext<R: StreamRequest> {
    handler: Rc<StreamRequestHandlerWrapper<R>>,
    /// Die noch ausstehenden Stream-Behaviors, das innerste zuerst; `run` nimmt das letzte, also äußerste.
    behaviors: Vec<Rc<StreamBehaviorWrapper<R>>>,
}

impl<R: StreamRequest> StreamNext<R> {
    /// Stream-Pipeline aus `behaviors`, in der Reihenfolge ihrer Registrierung, und `handler`; das zuerst registrierte liegt außen.
    pub(crate) fn new(
        handler: Rc<StreamRequestHandlerWrapper<R>>,
        behaviors: Vec<Rc<StreamBehaviorWrapper<R>>>,
    ) -> Self {
        Self {
            handler,
            behaviors: behaviors.into_iter().rev().collect(),
        }
    }

    /// Schickt `request` an den nächsten Schritt und liefert dessen Stream; der nächste Schritt läuft erst, wenn jemand den Stream liest.
    pub fn run(mut self, request: R) -> impl Stream<Item = StreamResult<R>> + use<R> {
        Deferred::new(move || match self.behaviors.pop() {
            Some(behavior) => behavior(request, self),
            None => (self.handler)(request),
        })
    }
}

// Von Hand statt `derive`: `derive(Clone)` verlangte `R: Clone`, obwohl nur die `Rc`-Felder geklont werden.
impl<R: StreamRequest> Clone for StreamNext<R> {
    fn clone(&self) -> Self {
        Self {
            handler: Rc::clone(&self.handler),
            behaviors: self.behaviors.clone(),
        }
    }
}

impl<R: StreamRequest> fmt::Debug for StreamNext<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamNext").finish_non_exhaustive()
    }
}
