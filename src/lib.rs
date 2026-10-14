//! Rust-Port von [MediatR](https://github.com/LuckyPennySoftware/MediatR) für In-Process-Messaging.

mod erased;
mod error;
mod mediator;
mod notification;
mod pipeline;
mod publish_strategy;
mod publisher;
mod registration;
mod request;
mod sender;

pub use error::SendError;
pub use mediator::{Mediator, WeakMediator};
pub use notification::{Notification, NotificationHandler};
pub use pipeline::{Next, PipelineBehavior, RequestPostProcessor, RequestPreProcessor};
pub use publish_strategy::{Concurrent, PublishStrategy, Sequential};
pub use publisher::Publisher;
pub use registration::{Handler, MediatorBuilder, NoHandlers, WithHandlers};
pub use request::{Request, RequestHandler};
pub use sender::Sender;
