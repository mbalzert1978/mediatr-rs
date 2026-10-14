//! Ein Request-Handler mit gelöschtem Typ.

use std::rc::Rc;

use super::LocalBoxFuture;
use crate::request::HandlerResult;
use crate::{Request, RequestHandler};

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
