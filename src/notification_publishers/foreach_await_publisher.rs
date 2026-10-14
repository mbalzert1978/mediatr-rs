//! Die Strategie, die Handler nacheinander laufen lässt.

use std::future::Future;

use crate::NotificationPublisher;

/// Lässt die Handler nacheinander laufen, in der Reihenfolge ihrer Registrierung; der erste Fehler stoppt, die Handler danach laufen nicht mehr.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ForeachAwaitPublisher;

impl NotificationPublisher for ForeachAwaitPublisher {
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
