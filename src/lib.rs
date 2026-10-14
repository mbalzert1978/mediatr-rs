//! Rust-Port von [MediatR](https://github.com/LuckyPennySoftware/MediatR) für In-Process-Messaging.

mod erased;
mod mediator;
mod notification;
mod pipeline;
mod request;
mod strategy;

pub use mediator::{
    Handler, Mediator, MediatorBuilder, NoHandlers, Publisher, SendError, Sender, WeakMediator,
    WithHandlers,
};
pub use notification::{Notification, NotificationHandler};
pub use pipeline::{Next, PipelineBehavior, RequestPreProcessor};
pub use request::{Request, RequestHandler};
pub use strategy::{Concurrent, PublishStrategy, Sequential};
