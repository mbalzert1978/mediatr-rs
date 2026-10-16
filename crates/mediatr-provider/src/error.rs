//! Der Fehler, wenn der Provider einen Dienst nicht liefern kann.

use std::error::Error;
use std::fmt;

/// Warum [`Provider::get`](crate::Provider::get) oder [`Provider::all`](crate::Provider::all) keinen Dienst liefert.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GetError {
    /// Kein Dienst vom Typ ist registriert.
    NoService,
    /// Die Bauanleitung des Typs läuft auf diesem Thread schon, sie hat nach ihrem eigenen Typ gefragt; oder zwei Threads bauen Singletons, die einander brauchen.
    Cycle,
    /// Ein `scoped`-Dienst wurde beim Provider selbst abgefragt statt in einem Scope, auch aus der Bauanleitung eines Singletons.
    ScopedFromRoot,
    /// Eine laufende Bauanleitung hat einen Dienst abgefragt, der nicht registriert ist, und den Fehler weitergegeben.
    NoDependency,
}

impl fmt::Display for GetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoService => f.write_str("kein Dienst von diesem Typ registriert"),
            Self::Cycle => f.write_str("die Bauanleitung dieses Typs läuft schon"),
            Self::ScopedFromRoot => {
                f.write_str("scoped-Dienst beim Provider selbst abgefragt statt in einem Scope")
            }
            Self::NoDependency => {
                f.write_str("die Bauanleitung hat einen nicht registrierten Dienst abgefragt")
            }
        }
    }
}

impl Error for GetError {}
