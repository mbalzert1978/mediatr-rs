//! Der Fehler, wenn der Provider einen Dienst nicht liefern kann.

use std::error::Error;
use std::fmt;

/// Warum [`Provider::get`](crate::Provider::get) keinen Dienst liefert.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GetError {
    /// Kein Dienst vom Typ ist registriert.
    NoService,
    /// Die Bauanleitung des Typs läuft auf diesem Thread schon; sie hat nach ihrem eigenen Typ gefragt.
    Cycle,
}

impl fmt::Display for GetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoService => f.write_str("kein Dienst von diesem Typ registriert"),
            Self::Cycle => f.write_str("die Bauanleitung dieses Typs läuft schon"),
        }
    }
}

impl Error for GetError {}
