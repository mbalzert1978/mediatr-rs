//! Gemeinsames Test-Setup für mehrere Testdateien.

use std::sync::{Arc, Mutex, PoisonError};

/// Das Protokoll, in das Handler, Behaviors und Test der Reihe nach schreiben.
#[derive(Clone, Default)]
pub struct Journal(Arc<Mutex<Vec<String>>>);

impl Journal {
    /// Hängt einen Eintrag hinten an.
    pub fn push(&self, entry: impl Into<String>) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(entry.into());
    }

    /// Gibt eine Kopie aller Einträge der Reihe nach zurück.
    pub fn entries(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}
