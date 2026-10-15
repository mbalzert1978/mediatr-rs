//! Ein Notification-Handler hinter einem Trait-Objekt.

use std::rc::Rc;

use super::LocalBoxFuture;
use crate::notification::NotificationResult;
use crate::{Notification, NotificationHandler};

/// Ein Notification-Handler für `N` als `Box<dyn Fn …>`: Die Closure hält den konkreten Handler in einem `Rc`, leiht sich ein `N` und meldet später `Ok(())` oder den Fehler.
// dyn: wie bei `LocalBoxFuture`, je `N`.
pub(crate) type NotificationHandlerWrapper<N> =
    Box<dyn for<'a> Fn(&'a N) -> LocalBoxFuture<'a, NotificationResult<N>>>;

/// Verpackt `handler` als [`NotificationHandlerWrapper`].
pub(crate) fn wrap_notification_handler<N, H>(handler: Rc<H>) -> NotificationHandlerWrapper<N>
where
    N: Notification + 'static,
    H: NotificationHandler<N> + 'static,
{
    Box::new(move |notification| Box::pin(notify(Rc::clone(&handler), notification)))
}

/// Ruft `handler` auf; die Future hält einen eigenen `Rc` auf den Handler und leiht sich nur die Notification.
async fn notify<N: Notification, H: NotificationHandler<N>>(
    handler: Rc<H>,
    notification: &N,
) -> NotificationResult<N> {
    handler.handle(notification).await
}
