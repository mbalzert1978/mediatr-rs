//! Tests für den Abschnitt „Request/Handler — `send`“ in `TESTLIST.md`.

use mediatr_rs::{Mediator, Request, RequestHandler};

struct Ping(String);

#[derive(Debug, PartialEq)]
struct Pong(String);

impl Request for Ping {
    type Response = Pong;
}

struct PingHandler;

impl RequestHandler<Ping> for PingHandler {
    async fn handle(&self, request: Ping) -> Pong {
        Pong(format!("{} Pong", request.0))
    }
}

#[tokio::test]
async fn send_returns_handler_response() {
    let mediator = Mediator::new(PingHandler);

    let response = mediator.send(Ping("Ping".to_owned())).await;

    assert_eq!(response, Pong("Ping Pong".to_owned()));
}
