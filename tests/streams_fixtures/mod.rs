use futures_core::Stream;
use futures_util::StreamExt;
use mediatr_rs::{
    Request, RequestHandler, SendError, StreamRequest, StreamRequestHandler, StreamSender,
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

/// Das Gästebuch: Es kennt die Gäste und meldet sie für jeden Tag einzeln.
pub struct GuestBook {
    pub guests: Vec<String>,
}

impl StreamRequestHandler<ListArrivals> for GuestBook {
    fn handle(
        &self,
        request: ListArrivals,
    ) -> impl Stream<Item = Result<String, SendError>> + use<> {
        let day = request.day;
        futures_util::stream::iter(
            self.guests
                .clone()
                .into_iter()
                .map(move |guest| Ok(format!("{guest} am Tag {day}"))),
        )
    }
}

/// Zählt die Gäste, die an diesem Tag anreisen.
pub struct CountArrivals {
    pub day: u32,
}

impl Request for CountArrivals {
    type Response = usize;
    type Error = SendError;
}

/// Die Anreisetafel: Sie fordert die Anreisen über `arrivals` als Stream an und zählt sie.
pub struct ArrivalBoard<M> {
    pub arrivals: M,
}

impl<M: StreamSender<ListArrivals>> RequestHandler<CountArrivals> for ArrivalBoard<M> {
    async fn handle(&self, request: CountArrivals) -> Result<usize, SendError> {
        let guests: Vec<_> = self
            .arrivals
            .create_stream(ListArrivals { day: request.day })
            .collect()
            .await;
        guests
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map(|guests| guests.len())
    }
}

/// Die Rezeption: Sie notiert jedes Nachschlagen der Anreisen im Protokoll und kennt noch keine Gäste.
pub struct FrontOffice {
    pub journal: Journal,
}

impl StreamRequestHandler<ListArrivals> for FrontOffice {
    fn handle(
        &self,
        request: ListArrivals,
    ) -> impl Stream<Item = Result<String, SendError>> + use<> {
        self.journal
            .push(format!("Anreisen für Tag {} nachgeschlagen", request.day));
        futures_util::stream::empty()
    }
}
