use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use mediatr_rs::{Request, RequestHandler, SendError};

pub struct Ping(pub String);

#[derive(Debug, PartialEq)]
pub struct Pong(pub String);

impl Request for Ping {
    type Response = Pong;
    type Error = SendError;
}

pub struct PingHandler;

impl RequestHandler<Ping> for PingHandler {
    async fn handle(&self, request: Ping) -> Result<Pong, SendError> {
        Ok(Pong(format!("{} Pong", request.0)))
    }
}

pub struct ReplacementPingHandler;

impl RequestHandler<Ping> for ReplacementPingHandler {
    async fn handle(&self, request: Ping) -> Result<Pong, SendError> {
        Ok(Pong(format!("{} Replacement", request.0)))
    }
}

pub struct VoidPing;

impl Request for VoidPing {
    type Response = ();
    type Error = SendError;
}

pub struct VoidPingHandler {
    pub called: Arc<AtomicBool>,
}

impl RequestHandler<VoidPing> for VoidPingHandler {
    async fn handle(&self, _request: VoidPing) -> Result<(), SendError> {
        self.called.store(true, Ordering::SeqCst);
        Ok(())
    }
}

pub struct Reject;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RejectError {
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

pub struct RejectHandler;

impl RequestHandler<Reject> for RejectHandler {
    async fn handle(&self, _request: Reject) -> Result<(), RejectError> {
        Err(RejectError::Rejected)
    }
}
