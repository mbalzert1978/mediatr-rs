//! Speichert zu jedem Request-Typ seinen Handler.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use crate::request::HandlerResult;
use crate::{HandlerProvider, Request, RequestHandler};

/// Ein Handler mit gelöschtem Typ: nimmt einen `R` und liefert später dessen Antwort oder Fehler.
// dyn: `handler` kennt nur `R`, nicht den Handler-Typ. Handler verschiedener Typen
// und ihre Futures liegen deshalb hinter einem gemeinsamen Typ je `R`.
pub(crate) type ErasedHandler<R> =
    Box<dyn Fn(R) -> Pin<Box<dyn Future<Output = HandlerResult<R>>>>>;

/// Provider, der zu jedem Request-Typ einen registrierten Handler hält.
#[derive(Debug, Default)]
pub struct Registry {
    // dyn: Hier liegen Handler verschiedener Typen zusammen.
    handlers: HashMap<TypeId, Box<dyn Any>>,
}

impl Registry {
    /// Registriert `handler` für Requests vom Typ `R`; ein früherer Handler für `R` fällt weg.
    pub fn register<R, H>(&mut self, handler: H)
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
}

impl<R: Request + 'static> HandlerProvider<R> for Registry {
    fn handler(&self) -> Option<impl RequestHandler<R>> {
        let erased = self
            .handlers
            .get(&TypeId::of::<R>())?
            .downcast_ref::<ErasedHandler<R>>()?;
        Some(move |request: R| erased(request))
    }
}

/// Ruft `handler` auf; die Future besitzt den Handler, damit sie ohne Leihe auskommt.
async fn call<R: Request, H: RequestHandler<R>>(handler: Rc<H>, request: R) -> HandlerResult<R> {
    handler.handle(request).await
}
