//! Der Mediator leitet einen Request an seinen Handler weiter.

use crate::{Request, RequestHandler};

/// Leitet Requests an den Handler weiter, mit dem er gebaut wurde.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Mediator<H> {
    handler: H,
}

impl<H> Mediator<H> {
    /// Baut einen Mediator, der Requests an `handler` weiterleitet.
    pub fn new(handler: H) -> Self {
        Self { handler }
    }

    /// Schickt `request` an seinen Handler und liefert dessen Antwort.
    pub async fn send<R>(&self, request: R) -> R::Response
    where
        R: Request,
        H: RequestHandler<R>,
    {
        self.handler.handle(request).await
    }
}
