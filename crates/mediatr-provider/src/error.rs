//! Der Fehler, wenn der Provider einen Dienst nicht liefern kann.

use std::error::Error;
use std::fmt;

/// Warum [`Provider::get`](crate::Provider::get) keinen Dienst liefert.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GetError {
    /// Kein Dienst vom Typ ist registriert.
    NoService,
}

impl fmt::Display for GetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoService => f.write_str("kein Dienst von diesem Typ registriert"),
        }
    }
}

impl Error for GetError {}
