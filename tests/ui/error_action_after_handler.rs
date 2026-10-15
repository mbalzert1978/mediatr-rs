//! Registriert eine Fehler-Aktion nach dem ersten Handler; das darf nicht kompilieren.

use mediatr_rs::{Handler, Mediator, Request, RequestErrorAction, RequestHandler, SendError};

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

/// Schlägt bei jedem Fehler Alarm.
struct BookingAlert;

impl<R: Request> RequestErrorAction<R> for BookingAlert {
    async fn execute(&self, _request: &R, _error: &R::Error) -> Result<(), R::Error> {
        Ok(())
    }
}

fn main() {
    let _mediator = Mediator::builder()
        .register(Handler::instance(OpenHotel))
        .open_request_error_action(BookingAlert)
        .build();
}
