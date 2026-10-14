//! Rust-Port von [MediatR](https://github.com/LuckyPennySoftware/MediatR) für In-Process-Messaging.

mod erased;
mod error;
mod mediator;
mod notification;
mod pipeline;
mod publisher;
mod registration;
mod request;
mod sender;
mod strategy;

pub use error::SendError;
pub use mediator::{Mediator, WeakMediator};
pub use notification::{Notification, NotificationHandler};
pub use pipeline::{Next, PipelineBehavior, RequestPostProcessor, RequestPreProcessor};
pub use publisher::Publisher;
pub use registration::{Handler, MediatorBuilder, NoHandlers, WithHandlers};
pub use request::{Request, RequestHandler};
pub use sender::Sender;
pub use strategy::{Concurrent, PublishStrategy, Sequential};
