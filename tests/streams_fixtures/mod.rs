use futures_core::Stream;
use mediatr_rs::{SendError, StreamRequest, StreamRequestHandler};

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
    ) -> impl Stream<Item = Result<String, SendError>> + 'static {
        let day = request.day;
        futures_util::stream::iter(
            self.guests
                .clone()
                .into_iter()
                .map(move |guest| Ok(format!("{guest} am Tag {day}"))),
        )
    }
}
