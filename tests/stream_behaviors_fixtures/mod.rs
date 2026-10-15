use std::cell::RefCell;
use std::rc::Rc;
use std::task::Poll;

use futures_core::Stream;
use futures_util::StreamExt;
use mediatr_rs::{
    SendError, StreamNext, StreamPipelineBehavior, StreamRequest, StreamRequestHandler,
};

/// Fragt die Gäste ab, die an diesem Tag anreisen.
pub struct ListArrivals {
    pub day: u32,
}

impl StreamRequest for ListArrivals {
    type Item = String;
    type Error = SendError;
}

/// Die Rezeption: Sie notiert jedes Nachschlagen der Anreisen im Protokoll und meldet dann ihre Gäste.
pub struct FrontOffice {
    pub guests: Vec<String>,
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl StreamRequestHandler<ListArrivals> for FrontOffice {
    fn handle(
        &self,
        request: ListArrivals,
    ) -> impl Stream<Item = Result<String, SendError>> + use<> {
        let day = request.day;
        self.journal
            .borrow_mut()
            .push(format!("Anreisen für Tag {day} nachgeschlagen"));
        futures_util::stream::iter(
            self.guests
                .clone()
                .into_iter()
                .map(move |guest| Ok(format!("{guest} am Tag {day}"))),
        )
    }
}

/// Der Concierge begleitet jede Abfrage und schreibt Beginn und Ende ins Protokoll.
pub struct Concierge {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl<R: StreamRequest + 'static> StreamPipelineBehavior<R> for Concierge {
    fn handle(
        &self,
        request: R,
        next: StreamNext<R>,
    ) -> impl Stream<Item = Result<R::Item, R::Error>> + use<R> {
        self.journal
            .borrow_mut()
            .push("Concierge vorher".to_owned());
        let journal = Rc::clone(&self.journal);
        next.run(request)
            .chain(futures_util::stream::poll_fn(move |_| {
                journal.borrow_mut().push("Concierge nachher".to_owned());
                Poll::Ready(None)
            }))
    }
}

/// Der Nachtportier begleitet jede Abfrage und schreibt Beginn und Ende ins Protokoll.
pub struct NightPorter {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl<R: StreamRequest + 'static> StreamPipelineBehavior<R> for NightPorter {
    fn handle(
        &self,
        request: R,
        next: StreamNext<R>,
    ) -> impl Stream<Item = Result<R::Item, R::Error>> + use<R> {
        self.journal
            .borrow_mut()
            .push("Nachtportier vorher".to_owned());
        let journal = Rc::clone(&self.journal);
        next.run(request)
            .chain(futures_util::stream::poll_fn(move |_| {
                journal.borrow_mut().push("Nachtportier nachher".to_owned());
                Poll::Ready(None)
            }))
    }
}

/// Was Concierge und Nachtportier, in dieser Reihenfolge registriert, um den Protokolleintrag `handled` eines Handlers herum schreiben.
pub fn concierge_and_night_porter_around(handled: &str) -> Vec<String> {
    [
        "Concierge vorher",
        "Nachtportier vorher",
        handled,
        "Nachtportier nachher",
        "Concierge nachher",
    ]
    .map(str::to_owned)
    .to_vec()
}
