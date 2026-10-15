//! Verpackt registrierte Handler oder Behaviors in Trait-Objekte (`Box<dyn Fn …>`), damit verschieden typisierte je Nachrichtentyp einen gemeinsamen Typ haben.

mod behavior;
mod notification;
mod request;

use std::future::Future;
use std::pin::Pin;

pub(crate) use behavior::{BehaviorWrapper, wrap_behavior};
pub(crate) use notification::{NotificationHandlerWrapper, wrap_notification_handler};
pub(crate) use request::{RequestHandlerWrapper, wrap_request_handler};

/// Eine Future hinter `dyn Future` in einer `Pin<Box<…>>`; sie läuft auf diesem Thread (nicht `Send`) und leiht höchstens `'a` lang.
// dyn: Handler, Behaviors und ihre Futures verschiedener Typen liegen hinter einem gemeinsamen Typ;
// `Sender::send` und `Publisher::publish` kennen nur den Nachrichtentyp, nicht den Handler-Typ;
// [`Next`] kennt nur den Request-Typ, nicht die Typen der Behaviors.
type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
