//! Rust-Port von [MediatR](https://github.com/LuckyPennySoftware/MediatR) für In-Process-Messaging.

mod error;
mod mediator;
mod notification;
mod notification_publishers;
mod pipeline;
mod publisher;
mod registration;
mod request;
mod sender;
mod wrappers;

pub use error::SendError;
pub use mediator::{Mediator, WeakMediator};
pub use notification::{Notification, NotificationHandler};
pub use notification_publishers::{ForeachAwaitPublisher, NotificationPublisher, WhenAllPublisher};
pub use pipeline::{Next, PipelineBehavior, RequestPostProcessor, RequestPreProcessor};
pub use publisher::Publisher;
pub use registration::{Handler, MediatorBuilder, NoHandlers, WithHandlers};
pub use request::{Request, RequestHandler};
pub use sender::Sender;
