//! Tests für den Abschnitt „Request/Handler — `send`“ in `TESTLIST.md`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use mediatr_rs::{Handler, Mediator, SendError, Sender, WeakMediator};

mod send_fixtures;

use send_fixtures::{
    Ping, PingHandler, Pong, Reject, RejectError, RejectHandler, ReplacementPingHandler, VoidPing,
    VoidPingHandler,
};

#[tokio::test]
async fn send_returns_handler_response() {
    let mediator = Mediator::builder()
        .register(Handler::instance(PingHandler))
        .build();

    let response = mediator.send(Ping("Ping".to_owned())).await.unwrap();

    assert_eq!(response, Pong("Ping Pong".to_owned()));
}

#[tokio::test]
async fn register_twice_keeps_last_handler() {
    let mediator = Mediator::builder()
        .register(Handler::instance(PingHandler))
        .register(Handler::instance(ReplacementPingHandler))
        .build();

    let response = mediator.send(Ping("Ping".to_owned())).await.unwrap();

    assert_eq!(response, Pong("Ping Replacement".to_owned()));
}

#[tokio::test]
async fn register_accepts_async_closure() {
    let mediator = Mediator::builder()
        .register(Handler::instance(async |request: Ping| {
            Ok(Pong(format!("{} Closure", request.0)))
        }))
        .build();

    let response = mediator.send(Ping("Ping".to_owned())).await.unwrap();

    assert_eq!(response, Pong("Ping Closure".to_owned()));
}

#[tokio::test]
async fn send_runs_handler_for_unit_response() {
    let called = Arc::new(AtomicBool::new(false));
    let mediator = Mediator::builder()
        .register(Handler::instance(VoidPingHandler {
            called: Arc::clone(&called),
        }))
        .build();

    mediator.send(VoidPing).await.unwrap();

    assert!(called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn send_reaches_only_handler_of_its_request_type() {
    let called = Arc::new(AtomicBool::new(false));
    let mediator = Mediator::builder()
        .register(Handler::instance(PingHandler))
        .register(Handler::instance(VoidPingHandler {
            called: Arc::clone(&called),
        }))
        .build();

    let response = mediator.send(Ping("Ping".to_owned())).await.unwrap();

    assert_eq!(response, Pong("Ping Pong".to_owned()));
    assert!(!called.load(Ordering::SeqCst));

    mediator.send(VoidPing).await.unwrap();

    assert!(called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn send_without_handler_returns_err() {
    let mediator = Mediator::builder().build();

    let response = mediator.send(Ping("Ping".to_owned())).await;

    assert_eq!(response, Err(SendError::NoHandler));
}

#[tokio::test]
async fn send_returns_handler_error() {
    let mediator = Mediator::builder()
        .register(Handler::instance(RejectHandler))
        .build();

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
    let (keep, handed_out) = mpsc::channel();
    let mediator = Mediator::builder()
        .register(Handler::singleton(move |sender: WeakMediator| {
            keep.send(sender.clone()).unwrap();
            PingHandler
        }))
        .build();
    mediator.send(Ping("Ping".to_owned())).await.unwrap();
    let sender = handed_out.try_recv().unwrap();
    drop(mediator);

    let result: Result<Pong, Box<dyn std::error::Error>> =
        async { Ok(sender.send(Ping("Ping".to_owned())).await?) }.await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "der Mediator ist weggefallen"
    );
}

#[tokio::test]
async fn register_twice_does_not_call_first_handler() {
    let first_called = Arc::new(AtomicBool::new(false));
    let second_called = Arc::new(AtomicBool::new(false));
    let mediator = Mediator::builder()
        .register(Handler::instance(VoidPingHandler {
            called: Arc::clone(&first_called),
        }))
        .register(Handler::instance(VoidPingHandler {
            called: Arc::clone(&second_called),
        }))
        .build();

    mediator.send(VoidPing).await.unwrap();

    assert_eq!(
        (
            first_called.load(Ordering::SeqCst),
            second_called.load(Ordering::SeqCst)
        ),
        (false, true)
    );
}
