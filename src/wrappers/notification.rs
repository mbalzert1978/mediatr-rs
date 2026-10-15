//! Ein Notification-Handler hinter einem Trait-Objekt.

use std::sync::Arc;

use super::LocalBoxFuture;
use crate::notification::NotificationResult;
use crate::{Notification, NotificationHandler};

/// Ein Notification-Handler für `N` als `Box<dyn Fn …>`: Die Closure hält den konkreten Handler in einem `Arc`, leiht sich ein `N` und meldet später `Ok(())` oder den Fehler.
// dyn: wie bei `LocalBoxFuture`, je `N`.
pub(crate) type NotificationHandlerWrapper<N> =
    Box<dyn for<'a> Fn(&'a N) -> LocalBoxFuture<'a, NotificationResult<N>> + Send + Sync>;

/// Verpackt `handler` als [`NotificationHandlerWrapper`].
pub(crate) fn wrap_notification_handler<N, H>(handler: Arc<H>) -> NotificationHandlerWrapper<N>
where
    N: Notification + 'static,
    H: NotificationHandler<N> + Send + Sync + 'static,
{
    Box::new(move |notification| Box::pin(notify(Arc::clone(&handler), notification)))
}

/// Ruft `handler` auf; die Future hält einen eigenen `Arc` auf den Handler und leiht sich nur die Notification.
async fn notify<N: Notification, H: NotificationHandler<N>>(
    handler: Arc<H>,
    notification: &N,
) -> NotificationResult<N> {
    handler.handle(notification).await
}
