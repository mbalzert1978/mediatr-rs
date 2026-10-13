//! Verträge für Notifications, die null bis viele Handler empfangen.

use std::future::Future;

use crate::SendError;

/// Eine Nachricht, die null bis viele Handler empfangen; keiner antwortet.
///
/// # Vertrag
///
/// `Error` ist der einzige Fehlertyp, den `publish` für diese Notification liefert.
/// `From<SendError>` liefert einen Wert, der „`publish` kann die Notification nicht zustellen“ bedeutet.
pub trait Notification {
    /// Typ des Fehlers, den `publish` für diese Notification liefert.
    type Error: From<SendError>;
}

/// Was ein Handler für `N` liefert: `Ok(())` oder den Fehler von `N`.
pub(crate) type NotificationResult<N> = Result<(), <N as Notification>::Error>;

/// Reagiert auf Notifications vom Typ `N`.
///
/// # Vertrag
///
/// `handle` liefert für jede Notification `Ok(())` oder einen Fehler und löst keine Panic aus.
pub trait NotificationHandler<N: Notification> {
    /// Reagiert auf `notification`.
    ///
    /// # Errors
    ///
    /// `N::Error`, wenn der Handler nicht auf `notification` reagieren kann.
    fn handle(&self, notification: &N) -> impl Future<Output = NotificationResult<N>>;
}
