//! Löscht den Typ eines Handlers, damit Handler verschiedener Typen im Provider liegen.

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use crate::notification::NotificationResult;
use crate::request::HandlerResult;
use crate::{Notification, NotificationHandler, Request, RequestHandler};

/// Ein Handler mit gelöschtem Typ: nimmt einen `R` und liefert später dessen Antwort oder Fehler.
// dyn: `Sender::send` kennt nur `R`, nicht den Handler-Typ. Handler verschiedener Typen
// und ihre Futures liegen deshalb hinter einem gemeinsamen Typ je `R`.
pub(crate) type ErasedHandler<R> =
    Box<dyn Fn(R) -> Pin<Box<dyn Future<Output = HandlerResult<R>>>>>;

/// Verpackt `handler` als [`ErasedHandler`].
pub(crate) fn erase<R, H>(handler: Rc<H>) -> ErasedHandler<R>
where
    R: Request + 'static,
    H: RequestHandler<R> + 'static,
{
    Box::new(move |request| Box::pin(call(Rc::clone(&handler), request)))
}

/// Ruft `handler` auf; die Future besitzt den Handler, damit sie ohne Leihe auskommt.
async fn call<R: Request, H: RequestHandler<R>>(handler: Rc<H>, request: R) -> HandlerResult<R> {
    handler.handle(request).await
}

/// Ein Notification-Handler mit gelöschtem Typ: leiht sich ein `N` und meldet später `Ok(())` oder den Fehler.
// dyn: aus demselben Grund wie bei `ErasedHandler`, je `N` statt je `R`.
pub(crate) type ErasedNotificationHandler<N> =
    Box<dyn for<'a> Fn(&'a N) -> Pin<Box<dyn Future<Output = NotificationResult<N>> + 'a>>>;

/// Verpackt `handler` als [`ErasedNotificationHandler`].
pub(crate) fn erase_notification_handler<N, H>(handler: Rc<H>) -> ErasedNotificationHandler<N>
where
    N: Notification + 'static,
    H: NotificationHandler<N> + 'static,
{
    Box::new(move |notification| Box::pin(notify(Rc::clone(&handler), notification)))
}

/// Ruft `handler` auf; die Future besitzt den Handler und leiht sich nur die Notification.
async fn notify<N: Notification, H: NotificationHandler<N>>(
    handler: Rc<H>,
    notification: &N,
) -> NotificationResult<N> {
    handler.handle(notification).await
}
