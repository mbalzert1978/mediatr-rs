//! Löscht den Typ eines Handlers, damit Handler verschiedener Typen im Provider liegen.

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use crate::request::HandlerResult;
use crate::{Request, RequestHandler};

/// Ein Handler mit gelöschtem Typ: nimmt einen `R` und liefert später dessen Antwort oder Fehler.
// dyn: `Sender::send` kennt nur `R`, nicht den Handler-Typ. Handler verschiedener Typen
// und ihre Futures liegen deshalb hinter einem gemeinsamen Typ je `R`.
pub(crate) type ErasedHandler<R> =
    Box<dyn Fn(R) -> Pin<Box<dyn Future<Output = HandlerResult<R>>>>>;

/// Verpackt `handler` als [`ErasedHandler`].
pub(crate) fn erase<R, H>(handler: H) -> ErasedHandler<R>
where
    R: Request + 'static,
    H: RequestHandler<R> + 'static,
{
    // Rc reicht, solange kein Testlisten-Punkt Threads verlangt; dann Arc.
    let handler = Rc::new(handler);
    Box::new(move |request| Box::pin(call(Rc::clone(&handler), request)))
}

/// Ruft `handler` auf; die Future besitzt den Handler, damit sie ohne Leihe auskommt.
async fn call<R: Request, H: RequestHandler<R>>(handler: Rc<H>, request: R) -> HandlerResult<R> {
    handler.handle(request).await
}
