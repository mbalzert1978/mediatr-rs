//! Rust-Port von [MediatR](https://github.com/LuckyPennySoftware/MediatR) für In-Process-Messaging.

mod mediator;
mod request;

pub use mediator::Mediator;
pub use request::{Request, RequestHandler};
