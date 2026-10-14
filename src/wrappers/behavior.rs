//! Ein Behavior mit gelöschtem Typ.

use std::rc::Rc;

use super::LocalBoxFuture;
use crate::request::HandlerResult;
use crate::{Next, PipelineBehavior, Request};

/// Ein Behavior mit gelöschtem Typ: nimmt einen `R` und den Rest der Pipeline und liefert später die Antwort oder den Fehler.
// dyn: wie bei `LocalBoxFuture`, je `R`.
pub(crate) type BehaviorWrapper<R> =
    Box<dyn Fn(R, Next<R>) -> LocalBoxFuture<'static, HandlerResult<R>>>;

/// Verpackt `behavior` als [`BehaviorWrapper`].
pub(crate) fn wrap_behavior<R, B>(behavior: Rc<B>) -> BehaviorWrapper<R>
where
    R: Request + 'static,
    B: PipelineBehavior<R> + 'static,
{
    Box::new(move |request, next| Box::pin(wrap(Rc::clone(&behavior), request, next)))
}

/// Ruft `behavior` auf; die Future besitzt das Behavior, damit sie ohne Leihe auskommt.
async fn wrap<R: Request, B: PipelineBehavior<R>>(
    behavior: Rc<B>,
    request: R,
    next: Next<R>,
) -> HandlerResult<R> {
    behavior.handle(request, next).await
}
