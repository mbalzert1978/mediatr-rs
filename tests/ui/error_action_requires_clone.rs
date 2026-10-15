//! Eine Fehler-Aktion ist registriert, der Request-Typ lässt sich nicht klonen; `send` darf nicht kompilieren.

use mediatr_rs::{
    Handler, Mediator, Request, RequestErrorAction, RequestHandler, SendError, Sender,
};

/// Bestellt ein Taxi an diese Hausnummer; lässt sich nicht klonen.
struct OrderTaxi {
    house: u32,
}

impl Request for OrderTaxi {
    type Response = u32;
    type Error = SendError;
}

/// Schickt Taxis los.
struct TaxiDispatch;

impl RequestHandler<OrderTaxi> for TaxiDispatch {
    async fn handle(&self, request: OrderTaxi) -> Result<u32, SendError> {
        Ok(request.house)
    }
}

/// Meldet jede gescheiterte Bestellung der Zentrale; braucht selbst kein `Clone`.
struct ControlRoom;

impl<R: Request> RequestErrorAction<R> for ControlRoom {
    async fn execute(&self, _request: &R, _error: &R::Error) -> Result<(), R::Error> {
        Ok(())
    }
}

fn main() {
    let mediator = Mediator::builder()
        .open_request_error_action(ControlRoom)
        .register(Handler::instance(TaxiDispatch))
        .build();
    let _taxi = mediator.send(OrderTaxi { house: 12 });
}
