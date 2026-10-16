use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Barrier};

use mediatr_provider::{GetError, Provider, WeakProvider};

pub struct Config;

pub struct Name(pub &'static str);

pub struct Greeting(pub String);

/// Eine Sitzung, die pro Scope neu entsteht.
pub struct Session;

/// Schreibt für die ganze Anwendung mit; `session` hält, was die Abfrage nach der Sitzung beim Bau ergab.
pub struct AuditLog {
    pub session: Result<Arc<Session>, GetError>,
}

/// Ein Bericht für eine Anfrage; `session` hält, was die Abfrage nach der Sitzung beim Bau ergab.
pub struct Report {
    pub session: Result<Arc<Session>, GetError>,
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

/// Einstellungen, die sich beim Bau auf übergeordnete Einstellungen stützen; `parent` hält, was die Abfrage danach ergab.
pub struct Settings {
    pub parent: Result<Arc<Settings>, GetError>,
}

/// Bauanleitung für `Settings`, die beim Provider nach übergeordneten `Settings` fragt.
pub fn settings_recipe(provider: &Provider) -> Settings {
    Settings {
        parent: provider.get::<Settings>(),
    }
}

/// Ein Ordner mit übergeordnetem Ordner und Vorlage, falls es sie gibt.
pub struct Folder {
    pub parent: Option<Arc<Folder>>,
    pub template: Option<Arc<Folder>>,
}

/// Ein Menüeintrag, der beim Bau alle Einträge seines Menüs sammelt.
pub struct MenuItem {
    pub siblings: Result<Vec<Arc<MenuItem>>, GetError>,
}

/// Ein Kurs mit allen Studierenden, die beim Bau bekannt sind.
pub struct Course {
    pub students: Result<Vec<Arc<Student>>, GetError>,
}

/// Eine Studentin mit allen Kursen, die beim Bau bekannt sind.
pub struct Student {
    pub courses: Result<Vec<Arc<Course>>, GetError>,
}

/// Ein Team mit seiner Leitung, falls es eine gibt.
pub struct Team {
    pub lead: Option<Arc<Employee>>,
}

/// Eine Mitarbeiterin mit ihrem Team, falls es eins gibt.
pub struct Employee {
    pub team: Option<Arc<Team>>,
}

/// Eine Verbindung zu einem Server, der nicht immer erreichbar ist.
pub struct Connection;

/// Liest Daten über eine Verbindung, falls es eine gibt.
pub struct Repository {
    pub connection: Option<Arc<Connection>>,
}

/// Liefert den Schalter für den Server (anfangs aus) und eine Bauanleitung für `Connection`, die mit einem Panic abbricht, solange er aus ist.
pub fn connection_recipe() -> (
    Arc<AtomicBool>,
    impl Fn(&Provider) -> Result<Connection, GetError>,
) {
    let server_up = Arc::new(AtomicBool::new(false));
    let reachable = Arc::clone(&server_up);
    let recipe = move |_: &Provider| {
        assert!(reachable.load(Ordering::SeqCst), "Server nicht erreichbar");
        Ok(Connection)
    };
    (server_up, recipe)
}

/// Liefert eine Bauanleitung für `Config` und den Zähler ihrer Läufe.
pub fn counting_recipe() -> (
    Arc<AtomicU32>,
    impl Fn(&Provider) -> Result<Config, GetError>,
) {
    let calls = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&calls);
    let recipe = move |_: &Provider| {
        counter.fetch_add(1, Ordering::SeqCst);
        Ok(Config)
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

/// Ein Tor für eine Bauanleitung: ihr erster Lauf meldet, dass er läuft, und wartet dann, bis `release` ihn weiterlässt.
pub struct Gate {
    started: Barrier,
    released: Barrier,
    entered: AtomicBool,
}

impl Gate {
    /// Wartet, bis der erste Lauf der Bauanleitung läuft.
    pub fn wait_started(&self) {
        self.started.wait();
    }

    /// Lässt den ersten Lauf der Bauanleitung weiterlaufen.
    pub fn release(&self) {
        self.released.wait();
    }

    /// Hält den ersten Lauf an, bis `release` ihn weiterlässt; jeder weitere Lauf geht durch.
    fn pass(&self) {
        if !self.entered.swap(true, Ordering::SeqCst) {
            self.started.wait();
            self.released.wait();
        }
    }
}

/// Liefert ein Tor und eine Bauanleitung für `Config`, deren erster Lauf an diesem Tor wartet.
pub fn gated_recipe() -> (Arc<Gate>, impl Fn(&Provider) -> Result<Config, GetError>) {
    let gate = Arc::new(Gate {
        started: Barrier::new(2),
        released: Barrier::new(2),
        entered: AtomicBool::new(false),
    });
    let pass = Arc::clone(&gate);
    let recipe = move |_: &Provider| {
        pass.pass();
        Ok(Config)
    };
    (gate, recipe)
}

/// Bauanleitung für ein Team, das nach seiner Leitung fragt; sie wartet an `both_building`, bis auch die Leitung gebaut wird, und fragt erst dann.
pub fn team_needing_lead(
    both_building: Arc<Barrier>,
) -> impl Fn(&Provider) -> Result<Team, GetError> {
    move |provider| {
        both_building.wait();
        Ok(Team {
            lead: provider.get::<Employee>().ok(),
        })
    }
}

/// Bauanleitung für eine Mitarbeiterin, die nach ihrem Team fragt; sie wartet an `both_building`, bis auch das Team gebaut wird, und fragt erst dann.
pub fn employee_needing_team(
    both_building: Arc<Barrier>,
) -> impl Fn(&Provider) -> Result<Employee, GetError> {
    move |provider| {
        both_building.wait();
        Ok(Employee {
            team: provider.get::<Team>().ok(),
        })
    }
}

/// Ein Teil der Daten der Anwendung; `router` hält, was die Abfrage nach dem Router beim Bau ergab, `None`, wenn der Teil keinen braucht.
pub struct Shard {
    pub router: Option<Result<Arc<Router>, GetError>>,
}

impl Shard {
    /// Der Fehler, den die Abfrage nach dem Router beim Bau ergab, falls einer.
    pub fn router_error(&self) -> Option<&GetError> {
        self.router
            .as_ref()
            .and_then(|router| router.as_ref().err())
    }
}

/// Verteilt Anfragen auf alle Teile; `shards` hält, was die Abfrage nach ihnen beim Bau ergab.
pub struct Router {
    pub shards: Result<Vec<Arc<Shard>>, GetError>,
}

impl Router {
    /// Der Fehler, den die Abfrage nach den Teilen beim Bau ergab, falls einer.
    pub fn shards_error(&self) -> Option<&GetError> {
        self.shards.as_ref().err()
    }
}

/// Bauanleitung für den ersten Teil: wartet an `both_shards`, bis auch der zweite gebaut wird, dann an `router_building`, bis der Router gebaut wird, und fragt erst dann nach ihm.
pub fn shard_needing_router(
    both_shards: Arc<Barrier>,
    router_building: Arc<Barrier>,
) -> impl Fn(&Provider) -> Result<Shard, GetError> {
    move |provider| {
        both_shards.wait();
        router_building.wait();
        Ok(Shard {
            router: Some(provider.get::<Router>()),
        })
    }
}

/// Bauanleitung für einen Teil ohne Router: wartet an `both_shards`, bis auch der erste gebaut wird, und ist dann fertig.
pub fn shard_without_router(
    both_shards: Arc<Barrier>,
) -> impl Fn(&Provider) -> Result<Shard, GetError> {
    move |_| {
        both_shards.wait();
        Ok(Shard { router: None })
    }
}

/// Bauanleitung für den Router: wartet an `router_building`, bis der erste Teil so weit ist, und sammelt dann alle Teile.
pub fn router_collecting_shards(
    router_building: Arc<Barrier>,
) -> impl Fn(&Provider) -> Result<Router, GetError> {
    move |provider| {
        router_building.wait();
        Ok(Router {
            shards: provider.all::<Shard>(),
        })
    }
}
