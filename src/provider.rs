//! Vertrag, über den der Mediator Handler bekommt.

use crate::{Request, RequestHandler};

/// Liefert den Handler für Requests vom Typ `R`.
///
/// # Vertrag
///
/// `handler` liefert `None` genau dann, wenn der Provider für `R` keinen Handler kennt,
/// und löst keine Panic aus.
pub trait HandlerProvider<R: Request> {
    /// Liefert den Handler für `R`, falls der Provider einen kennt.
    fn handler(&self) -> Option<impl RequestHandler<R>>;
}
