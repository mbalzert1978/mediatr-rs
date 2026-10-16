//! Ein Stream-Behavior hinter einem Trait-Objekt.

use std::sync::Arc;

use super::BoxStream;
use crate::stream_request::StreamResult;
use crate::{StreamNext, StreamPipelineBehavior, StreamRequest};

/// Ein Stream-Behavior für `R` als `Box<dyn Fn …>`: Die Closure hält das konkrete Behavior in einem `Arc`, nimmt einen `R` und den Rest der Stream-Pipeline und liefert einen Stream.
// dyn: wie bei `BoxFuture`, je `R`.
pub(crate) type StreamBehaviorWrapper<R> =
    Box<dyn Fn(R, StreamNext<R>) -> BoxStream<StreamResult<R>> + Send + Sync>;

/// Verpackt `behavior` als [`StreamBehaviorWrapper`].
pub(crate) fn wrap_stream_behavior<R, B>(behavior: Arc<B>) -> StreamBehaviorWrapper<R>
where
    R: StreamRequest,
    B: StreamPipelineBehavior<R> + 'static,
{
    Box::new(move |request, next| Box::pin(behavior.handle(request, next)))
}
