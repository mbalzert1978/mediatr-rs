//! Ein Request-Handler hinter einem Trait-Objekt.

use std::rc::Rc;

use super::LocalBoxFuture;
use crate::request::HandlerResult;
use crate::{Request, RequestHandler};

/// Ein Request-Handler für `R` als `Box<dyn Fn …>`: Die Closure hält den konkreten Handler in einem `Rc`, nimmt einen `R` und liefert später dessen Antwort oder Fehler.
// dyn: wie bei `LocalBoxFuture`, je `R`.
pub(crate) type RequestHandlerWrapper<R> =
    Box<dyn Fn(R) -> LocalBoxFuture<'static, HandlerResult<R>>>;

/// Verpackt `handler` als [`RequestHandlerWrapper`].
pub(crate) fn wrap_request_handler<R, H>(handler: Rc<H>) -> RequestHandlerWrapper<R>
where
    R: Request + 'static,
    H: RequestHandler<R> + 'static,
{
    Box::new(move |request| Box::pin(call(Rc::clone(&handler), request)))
}

/// Ruft `handler` auf; die Future hält einen eigenen `Rc` auf den Handler, damit sie ohne Leihe auskommt.
async fn call<R: Request, H: RequestHandler<R>>(handler: Rc<H>, request: R) -> HandlerResult<R> {
    handler.handle(request).await
}
