//! Speichert zu jedem Request-Typ seinen Handler.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use crate::{Request, RequestHandler};

/// Ein Handler mit gelöschtem Typ: nimmt einen `R` und liefert dessen Antwort später.
// dyn: `send` kennt nur `R`, nicht den Handler-Typ. Handler verschiedener Typen
// und ihre Futures liegen deshalb hinter einem gemeinsamen Typ je `R`.
pub(crate) type ErasedHandler<R> =
    Box<dyn Fn(R) -> Pin<Box<dyn Future<Output = <R as Request>::Response>>>>;

/// Handler verschiedener Typen, je Request-Typ einer.
#[derive(Debug, Default)]
pub(crate) struct Registry {
    // dyn: Hier liegen Handler verschiedener Typen zusammen.
    handlers: HashMap<TypeId, Box<dyn Any>>,
}

impl Registry {
    /// Legt `handler` für Requests vom Typ `R` ab.
    pub(crate) fn insert<R, H>(&mut self, handler: H)
    where
        R: Request + 'static,
        H: RequestHandler<R> + 'static,
    {
        // Rc reicht, solange kein Testlisten-Punkt Threads verlangt; dann Arc.
        let handler = Rc::new(handler);
        let erased: ErasedHandler<R> =
            Box::new(move |request| Box::pin(call(Rc::clone(&handler), request)));
        self.handlers.insert(TypeId::of::<R>(), Box::new(erased));
    }

    /// Liefert den Handler für Requests vom Typ `R`, falls einer abgelegt ist.
    pub(crate) fn get<R: Request + 'static>(&self) -> Option<&ErasedHandler<R>> {
        self.handlers.get(&TypeId::of::<R>())?.downcast_ref()
    }
}

/// Ruft `handler` auf; die Future besitzt den Handler, damit sie ohne Leihe auskommt.
async fn call<R: Request, H: RequestHandler<R>>(handler: Rc<H>, request: R) -> R::Response {
    handler.handle(request).await
}
