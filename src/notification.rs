//! Verträge für Notifications, die null bis viele Handler empfangen.

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
