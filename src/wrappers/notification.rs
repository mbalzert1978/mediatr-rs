//! Ein Notification-Handler mit gelöschtem Typ.

use std::rc::Rc;

use super::LocalBoxFuture;
use crate::notification::NotificationResult;
use crate::{Notification, NotificationHandler};

/// Ein Notification-Handler mit gelöschtem Typ: leiht sich ein `N` und meldet später `Ok(())` oder den Fehler.
// dyn: wie bei `LocalBoxFuture`, je `N`.
pub(crate) type ErasedNotificationHandler<N> =
    Box<dyn for<'a> Fn(&'a N) -> LocalBoxFuture<'a, NotificationResult<N>>>;

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
