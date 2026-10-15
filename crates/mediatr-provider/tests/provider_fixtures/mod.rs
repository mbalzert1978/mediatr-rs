use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use mediatr_provider::{Provider, WeakProvider};

pub struct Config;

pub struct Name(pub &'static str);

pub struct Greeting(pub String);

/// Eine Sitzung, die pro Scope neu entsteht; `0` ist ihre laufende Nummer.
pub struct Session(pub u32);

/// Schreibt für die ganze Anwendung mit; hält die Sitzung, die es beim Bau bekam.
pub struct AuditLog {
    pub session: Rc<Session>,
}

/// Meldet über den geteilten Schalter, ob der Dienst noch lebt, der ihn hält: Sein `Drop` setzt ihn auf `false`.
pub struct Alive(pub Arc<AtomicBool>);

impl Drop for Alive {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Ein Drucker im Netz, an den die Anwendung Aufträge schickt.
pub struct Printer {
    pub _alive: Alive,
}

/// Plant Aufgaben für die ganze Anwendung; merkt sich den Provider, um später Dienste zu holen.
pub struct Scheduler {
    pub _provider: WeakProvider,
    pub _alive: Alive,
}

/// Bündelt die Änderungen einer Anfrage; merkt sich ihren Scope, um später Dienste zu holen.
pub struct UnitOfWork {
    pub _scope: WeakProvider,
    pub _alive: Alive,
}

/// Ein Bericht, der seine Aufgaben über den Scheduler plant.
pub struct Report {
    pub _scheduler: Rc<Scheduler>,
    pub _alive: Alive,
}

/// Einstellungen, die sich beim Bau auf übergeordnete Einstellungen stützen, falls es welche gibt.
pub struct Settings {
    pub parent: Option<Rc<Settings>>,
}

/// Bauanleitung für `Settings`, die beim Provider nach übergeordneten `Settings` fragt.
pub fn settings_recipe(provider: &Provider) -> Settings {
    Settings {
        parent: provider.get::<Settings>(),
    }
}

/// Ein Ordner mit übergeordnetem Ordner und Vorlage, falls es sie gibt.
pub struct Folder {
    pub parent: Option<Rc<Folder>>,
    pub template: Option<Rc<Folder>>,
}

/// Ein Menüeintrag, der beim Bau alle Einträge seines Menüs sammelt.
pub struct MenuItem {
    pub siblings: Vec<Rc<MenuItem>>,
}

/// Ein Kurs mit allen Studierenden, die beim Bau bekannt sind.
pub struct Course {
    pub students: Vec<Rc<Student>>,
}

/// Eine Studentin mit allen Kursen, die beim Bau bekannt sind.
pub struct Student {
    pub courses: Vec<Rc<Course>>,
}

/// Ein Team mit seiner Leitung, falls es eine gibt.
pub struct Team {
    pub lead: Option<Rc<Employee>>,
}

/// Eine Mitarbeiterin mit ihrem Team, falls es eins gibt.
pub struct Employee {
    pub team: Option<Rc<Team>>,
}

/// Eine Verbindung zu einem Server, der nicht immer erreichbar ist.
pub struct Connection;

/// Liest Daten über eine Verbindung, falls es eine gibt.
pub struct Repository {
    pub connection: Option<Rc<Connection>>,
}

/// Liefert den Schalter für den Server (anfangs aus) und eine Bauanleitung für `Connection`, die mit einem Panic abbricht, solange er aus ist.
pub fn connection_recipe() -> (Arc<AtomicBool>, impl Fn(&Provider) -> Connection) {
    let server_up = Arc::new(AtomicBool::new(false));
    let reachable = Arc::clone(&server_up);
    let recipe = move |_: &Provider| {
        assert!(reachable.load(Ordering::SeqCst), "Server nicht erreichbar");
        Connection
    };
    (server_up, recipe)
}

/// Liefert eine Bauanleitung für `Config` und den Zähler ihrer Läufe.
pub fn counting_recipe() -> (Arc<AtomicU32>, impl Fn(&Provider) -> Config) {
    let calls = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&calls);
    let recipe = move |_: &Provider| {
        counter.fetch_add(1, Ordering::SeqCst);
        Config
    };
    (calls, recipe)
}

/// Eine Rechnung mit ihrer Überschrift.
pub struct Invoice {
    pub heading: String,
}

/// Ein druckfertiges Dokument mit seinem Titel.
pub struct Document(pub String);

/// Macht aus einer Rechnung ein Dokument mit ihrer Überschrift als Titel.
impl From<Invoice> for Document {
    fn from(invoice: Invoice) -> Self {
        Self(invoice.heading)
    }
}

/// Ein Lieferschein mit seiner Überschrift.
pub struct DeliveryNote {
    pub heading: String,
}

/// Macht aus einem Lieferschein ein Dokument mit seiner Überschrift als Titel.
impl From<DeliveryNote> for Document {
    fn from(note: DeliveryNote) -> Self {
        Self(note.heading)
    }
}

/// Ein Eintrag im Kassenbuch mit seinem Text.
pub struct LedgerEntry(pub String);

/// Bucht eine Rechnung mit ihrer Überschrift als Text ins Kassenbuch.
impl From<Invoice> for LedgerEntry {
    fn from(invoice: Invoice) -> Self {
        Self(invoice.heading)
    }
}
