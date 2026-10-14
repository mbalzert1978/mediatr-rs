//! Löscht den Typ registrierter Handler oder Behaviors, damit verschieden typisierte hinter einem gemeinsamen Typ zusammen liegen.

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use crate::notification::NotificationResult;
use crate::request::HandlerResult;
use crate::{Next, Notification, NotificationHandler, PipelineBehavior, Request, RequestHandler};

/// Eine Future mit gelöschtem Typ, die auf diesem Thread läuft (nicht `Send`) und höchstens `'a` lang leiht.
// dyn: Handler, Behaviors und ihre Futures verschiedener Typen liegen hinter einem gemeinsamen Typ;
// `Sender::send` und `Publisher::publish` kennen nur den Nachrichtentyp, nicht den Handler-Typ;
// [`Next`] kennt nur den Request-Typ, nicht die Typen der Behaviors.
type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// Ein Handler mit gelöschtem Typ: nimmt einen `R` und liefert später dessen Antwort oder Fehler.
// dyn: wie bei `LocalBoxFuture`, je `R`.
pub(crate) type ErasedHandler<R> = Box<dyn Fn(R) -> LocalBoxFuture<'static, HandlerResult<R>>>;

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

/// Ein Behavior mit gelöschtem Typ: nimmt einen `R` und den Rest der Pipeline und liefert später die Antwort oder den Fehler.
// dyn: wie bei `LocalBoxFuture`, je `R`.
pub(crate) type ErasedBehavior<R> =
    Box<dyn Fn(R, Next<R>) -> LocalBoxFuture<'static, HandlerResult<R>>>;

/// Verpackt das Behavior, das `pick` aus `behaviors` holt, als [`ErasedBehavior`].
pub(crate) fn erase_behavior<R, P, B>(behaviors: Rc<P>, pick: fn(&P) -> &B) -> ErasedBehavior<R>
where
    R: Request + 'static,
    P: 'static,
    B: PipelineBehavior<R> + 'static,
{
    Box::new(move |request, next| Box::pin(wrap(Rc::clone(&behaviors), pick, request, next)))
}

/// Ruft das Behavior auf, das `pick` aus `behaviors` holt; die Future besitzt die Behaviors, damit sie ohne Leihe auskommt.
async fn wrap<R: Request, P, B: PipelineBehavior<R>>(
    behaviors: Rc<P>,
    pick: fn(&P) -> &B,
    request: R,
    next: Next<R>,
) -> HandlerResult<R> {
    pick(&behaviors).handle(request, next).await
}
