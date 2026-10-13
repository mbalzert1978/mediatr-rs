//! Tests für den Abschnitt „Request/Handler — `send`“ in `TESTLIST.md`.

use std::cell::{Cell, OnceCell};
use std::rc::Rc;

use mediatr_rs::{Mediator, Request, RequestHandler, SendError, Sender, WeakMediator};

struct Ping(String);

#[derive(Debug, PartialEq)]
struct Pong(String);

impl Request for Ping {
    type Response = Pong;
    type Error = SendError;
}

struct PingHandler;

impl RequestHandler<Ping> for PingHandler {
    async fn handle(&self, request: Ping) -> Result<Pong, SendError> {
        Ok(Pong(format!("{} Pong", request.0)))
    }
}

#[tokio::test]
async fn send_returns_handler_response() {
    let mediator = Mediator::builder().register(PingHandler).build();

    let response = mediator.send(Ping("Ping".to_owned())).await.unwrap();

    assert_eq!(response, Pong("Ping Pong".to_owned()));
}

struct ReplacementPingHandler;

impl RequestHandler<Ping> for ReplacementPingHandler {
    async fn handle(&self, request: Ping) -> Result<Pong, SendError> {
        Ok(Pong(format!("{} Replacement", request.0)))
    }
}

#[tokio::test]
async fn register_twice_keeps_last_handler() {
    let mediator = Mediator::builder()
        .register(PingHandler)
        .register(ReplacementPingHandler)
        .build();

    let response = mediator.send(Ping("Ping".to_owned())).await.unwrap();

    assert_eq!(response, Pong("Ping Replacement".to_owned()));
}

#[tokio::test]
async fn register_accepts_async_closure() {
    let mediator = Mediator::builder()
        .register(async |request: Ping| Ok(Pong(format!("{} Closure", request.0))))
        .build();

    let response = mediator.send(Ping("Ping".to_owned())).await.unwrap();

    assert_eq!(response, Pong("Ping Closure".to_owned()));
}

struct VoidPing;

impl Request for VoidPing {
    type Response = ();
    type Error = SendError;
}

struct VoidPingHandler {
    called: Rc<Cell<bool>>,
}

impl RequestHandler<VoidPing> for VoidPingHandler {
    async fn handle(&self, _request: VoidPing) -> Result<(), SendError> {
        self.called.set(true);
        Ok(())
    }
}

#[tokio::test]
async fn send_runs_handler_for_unit_response() {
    let called = Rc::new(Cell::new(false));
    let mediator = Mediator::builder()
        .register(VoidPingHandler {
            called: Rc::clone(&called),
        })
        .build();

    mediator.send(VoidPing).await.unwrap();

    assert!(called.get());
}

#[tokio::test]
async fn send_reaches_only_handler_of_its_request_type() {
    let called = Rc::new(Cell::new(false));
    let mediator = Mediator::builder()
        .register(PingHandler)
        .register(VoidPingHandler {
            called: Rc::clone(&called),
        })
        .build();

    let response = mediator.send(Ping("Ping".to_owned())).await.unwrap();

    assert_eq!(response, Pong("Ping Pong".to_owned()));
    assert!(!called.get());

    mediator.send(VoidPing).await.unwrap();

    assert!(called.get());
}

#[tokio::test]
async fn send_without_handler_returns_err() {
    let mediator = Mediator::builder().build();

    let response = mediator.send(Ping("Ping".to_owned())).await;

    assert_eq!(response, Err(SendError::NoHandler));
}

struct Reject;

#[derive(Debug, PartialEq)]
enum RejectError {
    Undeliverable(SendError),
    Rejected,
}

impl From<SendError> for RejectError {
    fn from(error: SendError) -> Self {
        Self::Undeliverable(error)
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
    let mediator = Mediator::builder().register(RejectHandler).build();

    let response = mediator.send(Reject).await;

    assert_eq!(response, Err(RejectError::Rejected));
}

#[tokio::test]
async fn send_without_handler_converts_missing_handler_into_request_error() {
    let mediator = Mediator::builder().build();

    let response = mediator.send(Reject).await;

    assert_eq!(
        response,
        Err(RejectError::Undeliverable(SendError::NoHandler))
    );
}

#[tokio::test]
async fn send_error_propagates_as_boxed_error() {
    let mediator = Mediator::builder().build();

    let result: Result<Pong, Box<dyn std::error::Error>> =
        async { Ok(mediator.send(Ping("Ping".to_owned())).await?) }.await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "kein Handler für diesen Request-Typ registriert"
    );
}

#[tokio::test]
async fn mediator_dropped_error_propagates_as_boxed_error() {
    let handed_out = Rc::new(OnceCell::new());
    let keep = Rc::clone(&handed_out);
    let mediator = Mediator::builder()
        .singleton(move |sender: WeakMediator| {
            keep.get_or_init(|| sender.clone());
            PingHandler
        })
        .build();
    mediator.send(Ping("Ping".to_owned())).await.unwrap();
    let sender = handed_out.get().unwrap().clone();
    drop(mediator);

    let result: Result<Pong, Box<dyn std::error::Error>> =
        async { Ok(sender.send(Ping("Ping".to_owned())).await?) }.await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "der Mediator ist weggefallen"
    );
}
