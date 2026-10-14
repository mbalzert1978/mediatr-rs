//! Der Vertrag, über den ein Handler Notifications weitermeldet.

use std::future::Future;

use crate::Notification;

/// Meldet Notifications vom Typ `N` an alle ihre Handler; ein Handler bekommt einen Publisher per Konstruktor.
///
/// # Vertrag
///
/// `publish` liefert `Ok(())` oder einen Fehler und löst keine Panic aus.
pub trait Publisher<N: Notification> {
    /// Meldet `notification` an alle ihre Handler.
    ///
    /// # Errors
    ///
    /// Den Fehler eines Handlers, wie die [`PublishStrategy`](crate::PublishStrategy) des Mediators ihn liefert;
    /// bei [`Sequential`](crate::Sequential) ist es der erste, und die Handler danach laufen nicht mehr.
    /// Oder einen [`SendError`](crate::SendError) als `N::Error`, wenn `publish` die Notification nicht zustellen kann.
    fn publish(&self, notification: N) -> impl Future<Output = Result<(), N::Error>>;
}
