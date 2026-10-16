//! Verträge für Notifications, die null bis viele Handler empfangen.

use std::future::Future;

use crate::SendError;

/// Eine Nachricht, die null bis viele Handler empfangen; keiner antwortet.
///
/// # Vertrag
///
/// `Error` ist der einzige Fehlertyp, den `publish` für diese Notification liefert.
/// `From<SendError>` liefert einen Wert, der „`publish` kann die Notification nicht zustellen“ bedeutet.
/// Notification und Fehler sind `Send`, die Notification auch `Sync`: Die Futures der Handler leihen sie über ein `await`,
/// und `publish` liefert eine `Send`-Future.
pub trait Notification: Send + Sync {
    /// Typ des Fehlers, den `publish` für diese Notification liefert.
    type Error: From<SendError> + Send;
}

/// Was ein Handler für `N` liefert: `Ok(())` oder den Fehler von `N`.
pub(crate) type NotificationResult<N> = Result<(), <N as Notification>::Error>;

/// Reagiert auf Notifications vom Typ `N`.
///
/// # Vertrag
///
/// `handle` liefert für jede Notification `Ok(())` oder einen Fehler und löst keine Panic aus.
/// Der Handler ist `Send + Sync` und seine Future `Send`: Mehrere Threads teilen sich einen Handler.
pub trait NotificationHandler<N: Notification>: Send + Sync {
    /// Reagiert auf `notification`.
    ///
    /// # Errors
    ///
    /// `N::Error`, wenn der Handler nicht auf `notification` reagieren kann.
    fn handle(&self, notification: &N) -> impl Future<Output = NotificationResult<N>> + Send;
}
