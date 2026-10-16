//! Ein Request-Handler hinter einem Trait-Objekt.

use std::sync::Arc;

use super::BoxFuture;
use crate::request::HandlerResult;
use crate::{Request, RequestHandler};

/// Ein Request-Handler für `R` als `Box<dyn Fn …>`: Die Closure hält den konkreten Handler in einem `Arc`, nimmt einen `R` und liefert später dessen Antwort oder Fehler.
// dyn: wie bei `BoxFuture`, je `R`.
pub(crate) type RequestHandlerWrapper<R> =
    Box<dyn Fn(R) -> BoxFuture<'static, HandlerResult<R>> + Send + Sync>;

/// Verpackt `handler` als [`RequestHandlerWrapper`].
pub(crate) fn wrap_request_handler<R, H>(handler: Arc<H>) -> RequestHandlerWrapper<R>
where
    R: Request,
    H: RequestHandler<R> + 'static,
{
    Box::new(move |request| Box::pin(call(Arc::clone(&handler), request)))
}

/// Ruft `handler` auf; die Future hält einen eigenen `Arc` auf den Handler, damit sie ohne Leihe auskommt.
async fn call<R: Request, H: RequestHandler<R>>(handler: Arc<H>, request: R) -> HandlerResult<R> {
    handler.handle(request).await
}
