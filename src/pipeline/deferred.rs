//! Der Stream, der erst beim ersten Lesen entsteht.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

/// Der Stream, den `start` liefert; `start` läuft erst beim ersten Lesen.
pub(crate) struct Deferred<F, S> {
    /// Baut den Stream, solange er noch nicht gebaut ist.
    start: Option<F>,
    /// Der Stream, sobald `start` gelaufen ist.
    stream: Option<Pin<Box<S>>>,
}

impl<F, S> Deferred<F, S> {
    /// Stream, der beim ersten Lesen `start` aufruft und danach dessen Stream liefert.
    pub(crate) fn new(start: F) -> Self {
        Self {
            start: Some(start),
            stream: None,
        }
    }
}

// Von Hand: `Deferred` heftet `start` nicht an; der gebaute Stream liegt schon in einer `Pin<Box<…>>`.
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
