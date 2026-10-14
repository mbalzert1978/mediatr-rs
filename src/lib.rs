//! Rust-Port von [MediatR](https://github.com/LuckyPennySoftware/MediatR) für In-Process-Messaging.

mod erased;
mod mediator;
mod notification;
mod request;
mod strategy;

pub use mediator::{
    Handler, Mediator, MediatorBuilder, Publisher, SendError, Sender, WeakMediator,
};
pub use notification::{Notification, NotificationHandler};
pub use request::{Request, RequestHandler};
pub use strategy::{PublishStrategy, Sequential};
