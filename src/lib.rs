//! Rust-Port von [MediatR](https://github.com/LuckyPennySoftware/MediatR) für In-Process-Messaging.

mod mediator;
mod provider;
mod registry;
mod request;

pub use mediator::{Mediator, MediatorBuilder, MissingHandler};
pub use provider::HandlerProvider;
pub use registry::Registry;
pub use request::{Request, RequestHandler};
