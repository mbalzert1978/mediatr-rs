//! Tests für den Abschnitt „Mediator und Provider“ in `TESTLIST.md`.

use mediatr_rs::{HandlerProvider, Mediator, MissingHandler, Request, RequestHandler};

struct Ping(String);

#[derive(Debug, PartialEq)]
struct Pong(String);

impl Request for Ping {
    type Response = Pong;
    type Error = MissingHandler;
}

struct PingHandler;

impl RequestHandler<Ping> for PingHandler {
    async fn handle(&self, request: Ping) -> Result<Pong, MissingHandler> {
        Ok(Pong(format!("{} Pong", request.0)))
    }
}

struct PingProvider;

impl HandlerProvider<Ping> for PingProvider {
    fn handler(&self) -> Option<impl RequestHandler<Ping>> {
        Some(PingHandler)
    }
}

#[tokio::test]
async fn send_fetches_handler_from_provider() {
    let mediator = Mediator::builder(PingProvider).build();

    let response = mediator.send(Ping("Ping".to_owned())).await.unwrap();

    assert_eq!(response, Pong("Ping Pong".to_owned()));
}
