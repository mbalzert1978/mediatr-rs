//! Strategien, nach denen `publish` die Handler einer Notification laufen lässt.

use std::future::Future;

/// Wie [`Publisher::publish`](crate::Publisher::publish) die Handler einer Notification laufen lässt; ein [`Mediator`](crate::Mediator) bekommt sie als Typ-Parameter.
///
/// # Vertrag
///
/// `publish` wartet jeden Aufruf aus `calls` ab; erst nachdem ein Aufruf einen Fehler geliefert hat, darf es die übrigen auslassen.
/// Es liefert `Ok(())`, wenn jeder Aufruf `Ok(())` liefert, sonst den Fehler eines Aufrufs, und löst keine Panic aus.
pub trait PublishStrategy {
    /// Lässt `calls` laufen; `calls` enthält je einen Aufruf pro Handler, in der Reihenfolge ihrer Registrierung.
    ///
    /// # Errors
    ///
    /// Den Fehler eines Aufrufs.
    fn publish<E, F>(calls: Vec<F>) -> impl Future<Output = Result<(), E>>
    where
        F: Future<Output = Result<(), E>>;
}

/// Lässt die Handler nacheinander laufen, in der Reihenfolge ihrer Registrierung; der erste Fehler stoppt, die Handler danach laufen nicht mehr.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Sequential;

impl PublishStrategy for Sequential {
    async fn publish<E, F>(calls: Vec<F>) -> Result<(), E>
    where
        F: Future<Output = Result<(), E>>,
    {
        for call in calls {
            call.await?;
        }
        Ok(())
    }
}
