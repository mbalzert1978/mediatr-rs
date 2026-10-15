//! Registriert einen Fehler-Handler nach dem ersten Handler; das darf nicht kompilieren.

use mediatr_rs::{Handler, Mediator, Request, RequestErrorHandler, RequestHandler, SendError};

/// Bucht das Hotelzimmer mit dieser Nummer und liefert die Buchungsnummer.
#[derive(Clone)]
struct BookRoom {
    room: u32,
}

impl Request for BookRoom {
    type Response = u64;
    type Error = SendError;
}

/// Bucht Zimmer.
struct OpenHotel;

impl RequestHandler<BookRoom> for OpenHotel {
    async fn handle(&self, request: BookRoom) -> Result<u64, SendError> {
        Ok(u64::from(request.room))
    }
}

/// Reicht jeden Fehler weiter.
struct FrontDesk;

impl<R: Request> RequestErrorHandler<R> for FrontDesk {
    async fn handle(&self, _request: &R, error: R::Error) -> Result<R::Response, R::Error> {
        Err(error)
    }
}

fn main() {
    let _mediator = Mediator::builder()
        .register(Handler::instance(OpenHotel))
        .open_request_error_handler(FrontDesk)
        .build();
}
