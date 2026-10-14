//! Die Registrierung, aus der ein Mediator entsteht.

mod builder;
mod handler;

pub use builder::{MediatorBuilder, NoHandlers, WithHandlers};
pub use handler::Handler;
