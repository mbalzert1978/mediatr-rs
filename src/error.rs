//! Der Fehler, wenn eine Nachricht nicht zustellbar ist.

use std::error::Error;
use std::fmt;

/// Warum [`Sender::send`](crate::Sender::send) oder [`Publisher::publish`](crate::Publisher::publish) eine Nachricht nicht zustellen kann; beide liefern ihn als Fehlertyp der Nachricht.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SendError {
    /// Für den Request-Typ ist kein Handler registriert.
    NoHandler,
    /// Der Mediator, über den ein [`WeakMediator`](crate::WeakMediator) sendet oder meldet, ist weggefallen.
    MediatorDropped,
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHandler => f.write_str("kein Handler für diesen Request-Typ registriert"),
            Self::MediatorDropped => f.write_str("der Mediator ist weggefallen"),
        }
    }
}

impl Error for SendError {}
