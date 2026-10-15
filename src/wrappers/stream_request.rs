//! Ein Stream-Request-Handler hinter einem Trait-Objekt.

use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use futures_core::Stream;

use super::LocalBoxStream;
use crate::stream_request::StreamResult;
use crate::{StreamRequest, StreamRequestHandler};

/// Ein Stream-Request-Handler für `R` als `Box<dyn Fn …>`: Die Closure hält den konkreten Handler in einem `Rc`, nimmt einen `R` und liefert dessen Stream.
// dyn: wie bei `LocalBoxFuture`, je `R`.
pub(crate) type StreamRequestHandlerWrapper<R> = Box<dyn Fn(R) -> LocalBoxStream<StreamResult<R>>>;

/// Verpackt `handler` als [`StreamRequestHandlerWrapper`]; der Handler läuft erst, wenn jemand den Stream liest.
pub(crate) fn wrap_stream_request_handler<R, H>(handler: Rc<H>) -> StreamRequestHandlerWrapper<R>
where
    R: StreamRequest + 'static,
    H: StreamRequestHandler<R> + 'static,
{
    Box::new(move |request| {
        let handler = Rc::clone(&handler);
        Box::pin(Deferred {
            start: Some(move || handler.handle(request)),
            stream: None,
        })
    })
}

/// Der Stream, den `start` liefert; `start` läuft erst beim ersten Lesen.
struct Deferred<F, S> {
    /// Startet den Handler, solange er noch nicht gelaufen ist.
    start: Option<F>,
    /// Der Stream des Handlers, sobald er gelaufen ist.
    stream: Option<Pin<Box<S>>>,
}

// Von Hand: `Deferred` heftet `start` nicht an; der Stream des Handlers liegt schon in einer `Pin<Box<…>>`.
impl<F, S> Unpin for Deferred<F, S> {}

impl<F: FnOnce() -> S, S: Stream> Stream for Deferred<F, S> {
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let Some(start) = this.start.take() {
            this.stream = Some(Box::pin(start()));
        }
        this.stream
            .as_mut()
            .map_or(Poll::Ready(None), |stream| stream.as_mut().poll_next(cx))
    }
}
