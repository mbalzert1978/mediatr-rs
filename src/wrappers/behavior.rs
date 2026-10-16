//! Ein Behavior hinter einem Trait-Objekt.

use std::sync::Arc;

use super::BoxFuture;
use crate::request::HandlerResult;
use crate::{Next, PipelineBehavior, Request};

/// Ein Behavior für `R` als `Box<dyn Fn …>`: Die Closure hält das konkrete Behavior in einem `Arc`, nimmt einen `R` und den Rest der Pipeline und liefert später die Antwort oder den Fehler.
// dyn: wie bei `BoxFuture`, je `R`.
pub(crate) type BehaviorWrapper<R> =
    Box<dyn Fn(R, Next<R>) -> BoxFuture<'static, HandlerResult<R>> + Send + Sync>;

/// Verpackt `behavior` als [`BehaviorWrapper`].
pub(crate) fn wrap_behavior<R, B>(behavior: Arc<B>) -> BehaviorWrapper<R>
where
    R: Request,
    B: PipelineBehavior<R> + 'static,
{
    Box::new(move |request, next| Box::pin(wrap(Arc::clone(&behavior), request, next)))
}

/// Ruft `behavior` auf; die Future hält einen eigenen `Arc` auf das Behavior, damit sie ohne Leihe auskommt.
async fn wrap<R: Request, B: PipelineBehavior<R>>(
    behavior: Arc<B>,
    request: R,
    next: Next<R>,
) -> HandlerResult<R> {
    behavior.handle(request, next).await
}
