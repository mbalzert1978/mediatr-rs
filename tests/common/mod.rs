//! Gemeinsames Test-Setup für mehrere Testdateien.

use std::cell::RefCell;
use std::rc::Rc;

/// Das Protokoll, in das Handler, Behaviors und Test der Reihe nach schreiben.
#[derive(Clone, Default)]
pub struct Journal(Rc<RefCell<Vec<String>>>);

impl Journal {
    /// Hängt einen Eintrag hinten an.
    pub fn push(&self, entry: impl Into<String>) {
        self.0.borrow_mut().push(entry.into());
    }

    /// Gibt eine Kopie aller Einträge der Reihe nach zurück.
    pub fn entries(&self) -> Vec<String> {
        self.0.borrow().clone()
    }
}
