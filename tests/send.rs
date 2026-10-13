//! Tests für den Abschnitt „Request/Handler — `send`“ in `TESTLIST.md`.

use std::cell::Cell;
use std::rc::Rc;

use mediatr_rs::{Mediator, MissingHandler, Request, RequestHandler};

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

#[tokio::test]
async fn send_returns_handler_response() {
    let mut mediator = Mediator::default();
    mediator.register(PingHandler);

    let response = mediator.send(Ping("Ping".to_owned())).await.unwrap();

    assert_eq!(response, Pong("Ping Pong".to_owned()));
}

struct ReplacementPingHandler;

impl RequestHandler<Ping> for ReplacementPingHandler {
    async fn handle(&self, request: Ping) -> Result<Pong, MissingHandler> {
        Ok(Pong(format!("{} Replacement", request.0)))
    }
}

#[tokio::test]
async fn register_twice_keeps_last_handler() {
    let mut mediator = Mediator::default();
    mediator.register(PingHandler);
    mediator.register(ReplacementPingHandler);

    let response = mediator.send(Ping("Ping".to_owned())).await.unwrap();

    assert_eq!(response, Pong("Ping Replacement".to_owned()));
}

#[tokio::test]
async fn register_accepts_async_closure() {
    let mut mediator = Mediator::default();
    mediator.register(async |request: Ping| Ok(Pong(format!("{} Closure", request.0))));

    let response = mediator.send(Ping("Ping".to_owned())).await.unwrap();

    assert_eq!(response, Pong("Ping Closure".to_owned()));
}

struct VoidPing;

impl Request for VoidPing {
    type Response = ();
    type Error = MissingHandler;
}

struct VoidPingHandler {
    called: Rc<Cell<bool>>,
}

impl RequestHandler<VoidPing> for VoidPingHandler {
    async fn handle(&self, _request: VoidPing) -> Result<(), MissingHandler> {
        self.called.set(true);
        Ok(())
    }
}

#[tokio::test]
async fn send_runs_handler_for_unit_response() {
    let called = Rc::new(Cell::new(false));
    let mut mediator = Mediator::default();
    mediator.register(VoidPingHandler {
        called: Rc::clone(&called),
    });

    mediator.send(VoidPing).await.unwrap();

    assert!(called.get());
}

#[tokio::test]
async fn send_reaches_only_handler_of_its_request_type() {
    let called = Rc::new(Cell::new(false));
    let mut mediator = Mediator::default();
    mediator.register(PingHandler);
    mediator.register(VoidPingHandler {
        called: Rc::clone(&called),
    });

    let response = mediator.send(Ping("Ping".to_owned())).await.unwrap();

    assert_eq!(response, Pong("Ping Pong".to_owned()));
    assert!(!called.get());

    mediator.send(VoidPing).await.unwrap();

    assert!(called.get());
}

#[tokio::test]
async fn send_without_handler_returns_err() {
    let mediator = Mediator::default();

    let response = mediator.send(Ping("Ping".to_owned())).await;

    assert_eq!(response, Err(MissingHandler));
}

struct Reject;

#[derive(Debug, PartialEq)]
enum RejectError {
    MissingHandler,
    Rejected,
}

impl From<MissingHandler> for RejectError {
    fn from(_: MissingHandler) -> Self {
        Self::MissingHandler
    }
}

impl Request for Reject {
    type Response = ();
    type Error = RejectError;
}

struct RejectHandler;

impl RequestHandler<Reject> for RejectHandler {
    async fn handle(&self, _request: Reject) -> Result<(), RejectError> {
        Err(RejectError::Rejected)
    }
}

#[tokio::test]
async fn send_returns_handler_error() {
    let mut mediator = Mediator::default();
    mediator.register(RejectHandler);

    let response = mediator.send(Reject).await;

    assert_eq!(response, Err(RejectError::Rejected));
}

#[tokio::test]
async fn send_error_propagates_as_boxed_error() {
    let mediator = Mediator::default();

    let result: Result<Pong, Box<dyn std::error::Error>> =
        async { Ok(mediator.send(Ping("Ping".to_owned())).await?) }.await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "kein Handler für diesen Request-Typ registriert"
    );
}
