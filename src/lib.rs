//! Rust-Port von [MediatR](https://github.com/LuckyPennySoftware/MediatR) für In-Process-Messaging.

mod mediator;
mod registry;
mod request;

pub use mediator::{Mediator, MissingHandler};
pub use request::{Request, RequestHandler};
