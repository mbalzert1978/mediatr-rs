//! Strategien, nach denen `publish` die Handler einer Notification laufen lässt.

mod foreach_await_publisher;
mod when_all_publisher;

use std::future::Future;

pub use foreach_await_publisher::ForeachAwaitPublisher;
pub use when_all_publisher::WhenAllPublisher;

/// Wie [`Publisher::publish`](crate::Publisher::publish) die Handler einer Notification laufen lässt; ein [`Mediator`](crate::Mediator) bekommt sie als Typ-Parameter.
///
/// # Vertrag
///
/// `publish` wartet jeden Aufruf aus `calls` ab; erst nachdem ein Aufruf einen Fehler geliefert hat, darf es die übrigen auslassen.
/// Es liefert `Ok(())`, wenn jeder Aufruf `Ok(())` liefert, sonst den Fehler eines Aufrufs, und löst keine Panic aus.
pub trait NotificationPublisher {
    /// Lässt `calls` laufen; `calls` enthält je einen Aufruf pro Handler, in der Reihenfolge ihrer Registrierung.
    ///
    /// # Errors
    ///
    /// Den Fehler eines Aufrufs.
    fn publish<E, F>(calls: Vec<F>) -> impl Future<Output = Result<(), E>>
    where
        F: Future<Output = Result<(), E>>;
}
