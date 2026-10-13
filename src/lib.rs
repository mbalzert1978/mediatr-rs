//! Rust-Port von [MediatR](https://github.com/LuckyPennySoftware/MediatR) für In-Process-Messaging.

mod erased;
mod mediator;
mod request;

pub use mediator::{Mediator, MediatorBuilder, MissingHandler, Sender, WeakMediator};
pub use request::{Request, RequestHandler};
