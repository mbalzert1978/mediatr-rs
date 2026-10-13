//! Rust-Port von [MediatR](https://github.com/LuckyPennySoftware/MediatR) für In-Process-Messaging.

mod erased;
mod mediator;
mod request;

pub use mediator::{Mediator, MediatorBuilder, SendError, Sender, WeakMediator};
pub use request::{Request, RequestHandler};
