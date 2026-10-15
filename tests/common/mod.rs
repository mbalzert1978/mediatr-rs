//! Gemeinsames Test-Setup für mehrere Testdateien.

use std::cell::RefCell;
use std::rc::Rc;

/// Das Protokoll, in das Handler, Behaviors und Test der Reihe nach schreiben.
pub type Journal = Rc<RefCell<Vec<String>>>;
