//! Ein Stream-Request-Handler hinter einem Trait-Objekt.

use std::sync::Arc;

use super::BoxStream;
use crate::stream_request::StreamResult;
use crate::{StreamRequest, StreamRequestHandler};

/// Ein Stream-Request-Handler für `R` als `Box<dyn Fn …>`: Die Closure hält den konkreten Handler in einem `Arc`, nimmt einen `R` und liefert dessen Stream.
// dyn: wie bei `BoxFuture`, je `R`.
pub(crate) type StreamRequestHandlerWrapper<R> =
    Box<dyn Fn(R) -> BoxStream<StreamResult<R>> + Send + Sync>;

/// Verpackt `handler` als [`StreamRequestHandlerWrapper`].
pub(crate) fn wrap_stream_request_handler<R, H>(handler: Arc<H>) -> StreamRequestHandlerWrapper<R>
where
    R: StreamRequest,
    H: StreamRequestHandler<R> + 'static,
{
    Box::new(move |request| Box::pin(handler.handle(request)))
}
