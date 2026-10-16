use std::task::Poll;

use futures_core::Stream;
use futures_util::StreamExt;
use mediatr_rs::{
    Next, PipelineBehavior, Request, RequestHandler, SendError, StreamNext, StreamPipelineBehavior,
    StreamRequest, StreamRequestHandler,
};

use crate::common::Journal;

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
    pub journal: Journal,
}

impl StreamRequestHandler<ListArrivals> for FrontOffice {
    fn handle(
        &self,
        request: ListArrivals,
    ) -> impl Stream<Item = Result<String, SendError>> + use<> {
        let day = request.day;
        self.journal
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
    journal: &Journal,
    who: &'static str,
    inner: F,
) -> impl Stream<Item = S::Item> + use<F, S> {
    journal.push(format!("{who} vorher"));
    let journal = journal.clone();
    inner().chain(futures_util::stream::poll_fn(move |_| {
        journal.push(format!("{who} nachher"));
        Poll::Ready(None)
    }))
}

/// Der Concierge begleitet jede Abfrage und schreibt Beginn und Ende ins Protokoll.
pub struct Concierge {
    pub journal: Journal,
}

impl<R: StreamRequest> StreamPipelineBehavior<R> for Concierge {
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
    pub journal: Journal,
}

impl<R: StreamRequest> StreamPipelineBehavior<R> for NightPorter {
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
    pub journal: Journal,
}

impl StreamRequestHandler<ListDepartures> for DepartureBoard {
    fn handle(
        &self,
        request: ListDepartures,
    ) -> impl Stream<Item = Result<String, SendError>> + use<> {
        self.journal
            .push(format!("Abreisen für Tag {} nachgeschlagen", request.day));
        futures_util::stream::empty()
    }
}

/// Der Page begleitet nur Abfragen der Anreisen und schreibt Beginn und Ende ins Protokoll.
pub struct Bellboy {
    pub journal: Journal,
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
pub fn concierge(journal: &Journal) -> Concierge {
    Concierge {
        journal: journal.clone(),
    }
}

/// Page, der in `journal` schreibt.
pub fn bellboy(journal: &Journal) -> Bellboy {
    Bellboy {
        journal: journal.clone(),
    }
}

/// Nachtportier, der in `journal` schreibt.
pub fn night_porter(journal: &Journal) -> NightPorter {
    NightPorter {
        journal: journal.clone(),
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
    pub journal: Journal,
}

impl<R: Request> PipelineBehavior<R> for DoorLog {
    async fn handle(&self, request: R, next: Next<R>) -> Result<R::Response, R::Error> {
        self.journal.push("Tür auf");
        let response = next.run(request).await;
        self.journal.push("Tür zu");
        response
    }
}

/// Checkt den Gast in dieses Zimmer ein.
pub struct CheckIn {
    pub room: u32,
}

impl Request for CheckIn {
    type Response = ();
    type Error = SendError;
}

/// Der Check-in-Schalter: Er schreibt jeden Check-in ins Protokoll.
pub struct CheckInDesk {
    pub journal: Journal,
}

impl RequestHandler<CheckIn> for CheckInDesk {
    async fn handle(&self, request: CheckIn) -> Result<(), SendError> {
        self.journal
            .push(format!("Check-in Zimmer {}", request.room));
        Ok(())
    }
}
