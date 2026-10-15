use std::cell::RefCell;
use std::rc::Rc;
use std::task::Poll;

use futures_core::Stream;
use futures_util::StreamExt;
use mediatr_rs::{
    Next, PipelineBehavior, Request, SendError, StreamNext, StreamPipelineBehavior, StreamRequest,
    StreamRequestHandler,
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

/// Der Stream, den `inner` liefert; „`who` vorher“ steht im Protokoll, bevor `inner` läuft, „`who` nachher“ nach dem letzten Element.
fn journaled<F: FnOnce() -> S, S: Stream>(
    journal: &Rc<RefCell<Vec<String>>>,
    who: &'static str,
    inner: F,
) -> impl Stream<Item = S::Item> + use<F, S> {
    journal.borrow_mut().push(format!("{who} vorher"));
    let journal = Rc::clone(journal);
    inner().chain(futures_util::stream::poll_fn(move |_| {
        journal.borrow_mut().push(format!("{who} nachher"));
        Poll::Ready(None)
    }))
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
        journaled(&self.journal, "Concierge", || next.run(request))
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
        journaled(&self.journal, "Nachtportier", || next.run(request))
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

/// Fragt die Gäste ab, die an diesem Tag abreisen.
pub struct ListDepartures {
    pub day: u32,
}

impl StreamRequest for ListDepartures {
    type Item = String;
    type Error = SendError;
}

/// Die Abreisetafel: Sie notiert jedes Nachschlagen der Abreisen im Protokoll und meldet keine Gäste.
pub struct DepartureBoard {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl StreamRequestHandler<ListDepartures> for DepartureBoard {
    fn handle(
        &self,
        request: ListDepartures,
    ) -> impl Stream<Item = Result<String, SendError>> + use<> {
        self.journal
            .borrow_mut()
            .push(format!("Abreisen für Tag {} nachgeschlagen", request.day));
        futures_util::stream::empty()
    }
}

/// Der Page begleitet nur Abfragen der Anreisen und schreibt Beginn und Ende ins Protokoll.
pub struct Bellboy {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl StreamPipelineBehavior<ListArrivals> for Bellboy {
    fn handle(
        &self,
        request: ListArrivals,
        next: StreamNext<ListArrivals>,
    ) -> impl Stream<Item = Result<String, SendError>> + use<> {
        journaled(&self.journal, "Page", || next.run(request))
    }
}

/// Was Concierge, Page und Nachtportier, in dieser Reihenfolge registriert, um den Protokolleintrag `handled` eines Handlers herum schreiben.
pub fn concierge_bellboy_and_night_porter_around(handled: &str) -> Vec<String> {
    [
        "Concierge vorher",
        "Page vorher",
        "Nachtportier vorher",
        handled,
        "Nachtportier nachher",
        "Page nachher",
        "Concierge nachher",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// Concierge, der in `journal` schreibt.
pub fn concierge(journal: &Rc<RefCell<Vec<String>>>) -> Concierge {
    Concierge {
        journal: Rc::clone(journal),
    }
}

/// Page, der in `journal` schreibt.
pub fn bellboy(journal: &Rc<RefCell<Vec<String>>>) -> Bellboy {
    Bellboy {
        journal: Rc::clone(journal),
    }
}

/// Nachtportier, der in `journal` schreibt.
pub fn night_porter(journal: &Rc<RefCell<Vec<String>>>) -> NightPorter {
    NightPorter {
        journal: Rc::clone(journal),
    }
}

/// Der Empfang: Er stellt jeder Anreiseliste eine Begrüßung voran und hängt einen Abschied an.
pub struct Greeter;

impl StreamPipelineBehavior<ListArrivals> for Greeter {
    fn handle(
        &self,
        request: ListArrivals,
        next: StreamNext<ListArrivals>,
    ) -> impl Stream<Item = Result<String, SendError>> + use<> {
        futures_util::stream::iter([Ok("Begrüßung".to_owned())])
            .chain(next.run(request))
            .chain(futures_util::stream::iter([Ok("Abschied".to_owned())]))
    }
}

/// Die geschlossene Rezeption: Sie fragt die Anreisen beim nächsten Schritt an, liest sie aber nie und meldet keine Gäste.
pub struct ClosedDesk;

impl StreamPipelineBehavior<ListArrivals> for ClosedDesk {
    fn handle(
        &self,
        request: ListArrivals,
        next: StreamNext<ListArrivals>,
    ) -> impl Stream<Item = Result<String, SendError>> + use<> {
        drop(next.run(request));
        futures_util::stream::empty()
    }
}

/// Die Türprotokollierung: Sie umschließt jeden Request und schreibt Öffnen und Schließen ins Protokoll.
pub struct DoorLog {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl<R: Request> PipelineBehavior<R> for DoorLog {
    async fn handle(&self, request: R, next: Next<R>) -> Result<R::Response, R::Error> {
        self.journal.borrow_mut().push("Tür auf".to_owned());
        let response = next.run(request).await;
        self.journal.borrow_mut().push("Tür zu".to_owned());
        response
    }
}
