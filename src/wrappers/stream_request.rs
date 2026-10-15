//! Ein Stream-Request-Handler hinter einem Trait-Objekt.

use std::rc::Rc;

use super::LocalBoxStream;
use crate::stream_request::StreamResult;
use crate::{StreamRequest, StreamRequestHandler};

/// Ein Stream-Request-Handler für `R` als `Box<dyn Fn …>`: Die Closure hält den konkreten Handler in einem `Rc`, nimmt einen `R` und liefert dessen Stream.
// dyn: wie bei `LocalBoxFuture`, je `R`.
pub(crate) type StreamRequestHandlerWrapper<R> = Box<dyn Fn(R) -> LocalBoxStream<StreamResult<R>>>;

/// Verpackt `handler` als [`StreamRequestHandlerWrapper`].
pub(crate) fn wrap_stream_request_handler<R, H>(handler: Rc<H>) -> StreamRequestHandlerWrapper<R>
where
    R: StreamRequest + 'static,
    H: StreamRequestHandler<R> + 'static,
{
    Box::new(move |request| Box::pin(handler.handle(request)))
}
