//! Registriert ein Stream-Behavior nach dem ersten Handler; das darf nicht kompilieren.

use futures_core::Stream;
use mediatr_rs::{
    Handler, Mediator, SendError, StreamNext, StreamPipelineBehavior, StreamRequest,
    StreamRequestHandler,
};

/// Fragt die Ausleihen dieses Lesers ab.
struct ListLoans {
    reader: u32,
}

impl StreamRequest for ListLoans {
    type Item = u32;
    type Error = SendError;
}

/// Die Bibliothek kennt keine Ausleihen.
struct Library;

impl StreamRequestHandler<ListLoans> for Library {
    fn handle(&self, request: ListLoans) -> impl Stream<Item = Result<u32, SendError>> + use<> {
        futures_util::stream::iter([Ok(request.reader)])
    }
}

/// Reicht jede Abfrage unverändert weiter.
struct Librarian;

impl<R: StreamRequest> StreamPipelineBehavior<R> for Librarian {
    fn handle(
        &self,
        request: R,
        next: StreamNext<R>,
    ) -> impl Stream<Item = Result<R::Item, R::Error>> + use<R> {
        next.run(request)
    }
}

fn main() {
    let _mediator = Mediator::builder()
        .register_stream(Handler::instance(Library))
        .open_stream_behavior(Librarian)
        .build();
}
