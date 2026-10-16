//! Ein Fehler-Handler ist registriert, der Request-Typ mit Handler lässt sich nicht klonen; das Programm darf nicht kompilieren.

use mediatr_rs::{
    Handler, Mediator, Request, RequestErrorHandler, RequestHandler, SendError, Sender,
};

/// Reserviert einen Tisch für so viele Gäste; lässt sich nicht klonen.
struct ReserveTable {
    guests: u32,
}

impl Request for ReserveTable {
    type Response = u32;
    type Error = SendError;
}

/// Vergibt Tische.
struct Restaurant;

impl RequestHandler<ReserveTable> for Restaurant {
    async fn handle(&self, request: ReserveTable) -> Result<u32, SendError> {
        Ok(request.guests)
    }
}

/// Reicht jeden Fehler weiter; braucht selbst kein `Clone`.
struct Maitre;

impl<R: Request> RequestErrorHandler<R> for Maitre {
    async fn handle(&self, _request: &R, error: R::Error) -> Result<R::Response, R::Error> {
        Err(error)
    }
}

fn main() {
    let mediator = Mediator::builder()
        .open_request_error_handler(Maitre)
        .register(Handler::instance(Restaurant))
        .build();
    let _table = mediator.send(ReserveTable { guests: 4 });
}
