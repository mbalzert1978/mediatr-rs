//! Tests für den Abschnitt „Provider-Crate `mediatr-provider`“ in `TESTLIST.md`.

use std::cell::{Cell, RefCell};
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

use mediatr_provider::{Provider, Registration, WeakProvider};

struct Config;

struct Name(&'static str);

#[test]
fn instance_returns_same_object_on_every_get() {
    let provider = Provider::builder()
        .register(Registration::instance(Config))
        .build();

    let first = provider.get::<Config>().unwrap();
    let second = provider.get::<Config>().unwrap();

    assert!(Rc::ptr_eq(&first, &second));
}

#[test]
fn get_without_registration_returns_none() {
    let provider = Provider::builder().build();

    let service = provider.get::<Config>();

    assert!(service.is_none());
}

#[test]
fn instance_twice_returns_last_registered_service() {
    let provider = Provider::builder()
        .register(Registration::instance(Name("alt")))
        .register(Registration::instance(Name("neu")))
        .build();

    let name = provider.get::<Name>().unwrap();

    assert_eq!(name.0, "neu");
}

#[test]
fn all_returns_every_instance_in_registration_order() {
    let provider = Provider::builder()
        .register(Registration::instance(Name("alt")))
        .register(Registration::instance(Name("neu")))
        .build();

    let names: Vec<_> = provider.all::<Name>().iter().map(|name| name.0).collect();

    assert_eq!(names, ["alt", "neu"]);
}

#[test]
fn all_without_registration_returns_empty_list() {
    let provider = Provider::builder()
        .register(Registration::instance(Name("alt")))
        .build();

    let configs = provider.all::<Config>();

    assert!(configs.is_empty());
}

#[test]
fn transient_all_runs_recipe_on_every_call() {
    let (calls, recipe) = counting_recipe();
    let provider = Provider::builder()
        .register(Registration::transient(recipe))
        .build();

    drop(provider.all::<Config>());
    drop(provider.all::<Config>());

    assert_eq!(calls.get(), 2);
}

#[test]
fn singleton_all_returns_same_object_as_get() {
    let provider = Provider::builder()
        .register(Registration::singleton(|_| Config))
        .build();

    let single = provider.get::<Config>().unwrap();
    let all = provider.all::<Config>();

    assert!(matches!(all.as_slice(), [config] if Rc::ptr_eq(config, &single)));
}

#[test]
fn scoped_all_returns_same_object_as_get_within_scope() {
    let provider = Provider::builder()
        .register(Registration::scoped(|_| Config))
        .build();
    let first = provider.scope();
    let second = provider.scope();

    let single = first.get::<Config>().unwrap();
    let same_scope = first.all::<Config>();
    let other_scope = second.all::<Config>();

    assert!(matches!(same_scope.as_slice(), [config] if Rc::ptr_eq(config, &single)));
    assert!(matches!(other_scope.as_slice(), [config] if !Rc::ptr_eq(config, &single)));
}

#[test]
fn transient_runs_recipe_on_every_get() {
    let (calls, recipe) = counting_recipe();
    let provider = Provider::builder()
        .register(Registration::transient(recipe))
        .build();

    provider.get::<Config>().unwrap();
    provider.get::<Config>().unwrap();

    assert_eq!(calls.get(), 2);
}

#[test]
fn singleton_runs_recipe_once_on_first_get() {
    let (calls, recipe) = counting_recipe();
    let provider = Provider::builder()
        .register(Registration::singleton(recipe))
        .build();
    assert_eq!(calls.get(), 0);

    provider.get::<Config>().unwrap();
    provider.get::<Config>().unwrap();

    assert_eq!(calls.get(), 1);
}

#[test]
fn scoped_runs_recipe_once_per_scope() {
    let (calls, recipe) = counting_recipe();
    let provider = Provider::builder()
        .register(Registration::scoped(recipe))
        .build();
    let first = provider.scope();
    let second = provider.scope();

    first.get::<Config>().unwrap();
    first.get::<Config>().unwrap();
    second.get::<Config>().unwrap();

    assert_eq!(calls.get(), 2);
}

#[test]
fn singleton_returns_same_object_in_new_scope() {
    let provider = Provider::builder()
        .register(Registration::singleton(|_| Config))
        .build();

    let first = provider.get::<Config>().unwrap();
    let second = provider.scope().get::<Config>().unwrap();

    assert!(Rc::ptr_eq(&first, &second));
}

struct Greeting(String);

#[test]
fn recipe_gets_other_service_from_provider() {
    let provider = Provider::builder()
        .register(Registration::instance(Name("Welt")))
        .register(Registration::transient(|provider| {
            Greeting(format!("Hallo {}", provider.get::<Name>().unwrap().0))
        }))
        .build();

    let greeting = provider.get::<Greeting>().unwrap();

    assert_eq!(greeting.0, "Hallo Welt");
}

/// Eine Sitzung, die pro Scope neu entsteht; `0` ist ihre laufende Nummer.
struct Session(u32);

/// Schreibt für die ganze Anwendung mit; hält die Sitzung, die es beim Bau bekam.
struct AuditLog {
    session: Rc<Session>,
}

#[test]
fn singleton_built_from_scope_gets_scoped_service_of_provider() {
    let opened = Cell::new(0_u32);
    let provider = Provider::builder()
        .register(Registration::scoped(move |_| {
            opened.set(opened.get().saturating_add(1));
            Session(opened.get())
        }))
        .register(Registration::singleton(|provider| AuditLog {
            session: provider.get::<Session>().unwrap(),
        }))
        .build();

    let log = provider.scope().get::<AuditLog>().unwrap();
    let provider_session = provider.get::<Session>().unwrap();

    assert_eq!(log.session.0, provider_session.0);
}

#[test]
fn two_scoped_registrations_of_same_type_build_one_object_each() {
    let scope = Provider::builder()
        .register(Registration::scoped(|_| Name("haupt")))
        .register(Registration::scoped(|_| Name("ersatz")))
        .build()
        .scope();

    let names: Vec<_> = scope.all::<Name>().iter().map(|name| name.0).collect();

    assert_eq!(names, ["haupt", "ersatz"]);
}

#[test]
fn cloned_scope_returns_same_scoped_service() {
    let (calls, recipe) = counting_recipe();
    let scope = Provider::builder()
        .register(Registration::scoped(recipe))
        .build()
        .scope();
    let clone = scope.clone();

    scope.get::<Config>().unwrap();
    clone.get::<Config>().unwrap();

    assert_eq!(calls.get(), 1);
}

#[test]
fn upgraded_weak_provider_returns_same_singleton() {
    let provider = Provider::builder()
        .register(Registration::singleton(|_| Config))
        .build();
    let weak = provider.downgrade();

    let upgraded = weak.upgrade().unwrap();

    assert!(Rc::ptr_eq(
        &provider.get::<Config>().unwrap(),
        &upgraded.get::<Config>().unwrap()
    ));
}

/// Meldet über den geteilten Schalter, ob der Dienst noch lebt, der ihn hält: Sein `Drop` setzt ihn auf `false`.
struct Alive(Rc<Cell<bool>>);

impl Drop for Alive {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// Ein Drucker im Netz, an den die Anwendung Aufträge schickt.
struct Printer {
    _alive: Alive,
}

#[test]
fn both_instances_of_same_type_are_released_with_provider() {
    let office = Rc::new(Cell::new(true));
    let lobby = Rc::new(Cell::new(true));
    let provider = Provider::builder()
        .register(Registration::instance(Printer {
            _alive: Alive(Rc::clone(&office)),
        }))
        .register(Registration::instance(Printer {
            _alive: Alive(Rc::clone(&lobby)),
        }))
        .build();

    drop(provider);

    assert!(!office.get() && !lobby.get());
}

/// Plant Aufgaben für die ganze Anwendung; merkt sich den Provider, um später Dienste zu holen.
struct Scheduler {
    _provider: WeakProvider,
    _alive: Alive,
}

#[test]
fn singleton_holding_weak_provider_is_released_with_provider() {
    let running = Rc::new(Cell::new(true));
    let flag = Rc::clone(&running);
    let provider = Provider::builder()
        .register(Registration::singleton(move |provider| Scheduler {
            _provider: provider.downgrade(),
            _alive: Alive(Rc::clone(&flag)),
        }))
        .build();
    provider.get::<Scheduler>().unwrap();

    drop(provider);

    assert!(!running.get());
}

#[test]
fn upgrade_after_provider_dropped_returns_none() {
    let provider = Provider::builder().build();
    let weak = provider.downgrade();

    drop(provider);

    assert!(weak.upgrade().is_none());
}

#[test]
fn upgraded_weak_scope_returns_same_scoped_service() {
    let scope = Provider::builder()
        .register(Registration::scoped(|_| Config))
        .build()
        .scope();
    let weak = scope.downgrade();

    let upgraded = weak.upgrade().unwrap();

    assert!(Rc::ptr_eq(
        &scope.get::<Config>().unwrap(),
        &upgraded.get::<Config>().unwrap()
    ));
}

#[test]
fn upgrade_after_scope_dropped_returns_none() {
    let provider = Provider::builder().build();
    let scope = provider.scope();
    let weak = scope.downgrade();

    drop(scope);

    assert!(weak.upgrade().is_none());
}

/// Bündelt die Änderungen einer Anfrage; merkt sich ihren Scope, um später Dienste zu holen.
struct UnitOfWork {
    _scope: WeakProvider,
    _alive: Alive,
}

#[test]
fn scoped_holding_weak_scope_is_released_with_scope() {
    let open = Rc::new(Cell::new(true));
    let flag = Rc::clone(&open);
    let provider = Provider::builder()
        .register(Registration::scoped(move |scope| UnitOfWork {
            _scope: scope.downgrade(),
            _alive: Alive(Rc::clone(&flag)),
        }))
        .build();
    let scope = provider.scope();
    scope.get::<UnitOfWork>().unwrap();

    drop(scope);

    assert!(!open.get());
}

/// Ein Bericht, der seine Aufgaben über den Scheduler plant.
struct Report {
    _scheduler: Rc<Scheduler>,
    _alive: Alive,
}

#[test]
fn scoped_service_and_its_singleton_with_weak_provider_are_released_with_provider() {
    let running = Rc::new(Cell::new(true));
    let open = Rc::new(Cell::new(true));
    let (running_flag, open_flag) = (Rc::clone(&running), Rc::clone(&open));
    let provider = Provider::builder()
        .register(Registration::singleton(move |provider| Scheduler {
            _provider: provider.downgrade(),
            _alive: Alive(Rc::clone(&running_flag)),
        }))
        .register(Registration::scoped(move |provider| Report {
            _scheduler: provider.get::<Scheduler>().unwrap(),
            _alive: Alive(Rc::clone(&open_flag)),
        }))
        .build();
    provider.get::<Report>().unwrap();

    drop(provider);

    assert!(!running.get() && !open.get());
}

/// Einstellungen, die sich beim Bau auf übergeordnete Einstellungen stützen, falls es welche gibt.
struct Settings {
    parent: Option<Rc<Settings>>,
}

/// Bauanleitung für `Settings`, die beim Provider nach übergeordneten `Settings` fragt.
fn settings_recipe(provider: &Provider) -> Settings {
    Settings {
        parent: provider.get::<Settings>(),
    }
}

#[test]
fn recipe_asking_for_own_type_gets_none() {
    let provider = Provider::builder()
        .register(Registration::transient(settings_recipe))
        .build();

    let settings = provider.get::<Settings>().unwrap();

    assert!(settings.parent.is_none());
}

#[test]
fn first_of_two_recipes_asking_for_own_type_through_all_gets_none() {
    let provider = Provider::builder()
        .register(Registration::transient(settings_recipe))
        .register(Registration::transient(|_| Settings { parent: None }))
        .build();

    let settings = provider.all::<Settings>();

    assert!(matches!(settings.as_slice(), [first, _] if first.parent.is_none()));
}

#[test]
fn singleton_recipe_asking_for_own_type_gets_none() {
    let provider = Provider::builder()
        .register(Registration::singleton(settings_recipe))
        .build();

    let settings = provider.get::<Settings>().unwrap();

    assert!(settings.parent.is_none());
}

#[test]
fn scoped_recipe_asking_for_own_type_gets_none() {
    let provider = Provider::builder()
        .register(Registration::scoped(settings_recipe))
        .build();

    let settings = provider.get::<Settings>().unwrap();

    assert!(settings.parent.is_none());
}

/// Ein Ordner mit übergeordnetem Ordner und Vorlage, falls es sie gibt.
struct Folder {
    parent: Option<Rc<Folder>>,
    template: Option<Rc<Folder>>,
}

#[test]
fn recipe_asking_twice_for_own_type_gets_none_both_times() {
    let provider = Provider::builder()
        .register(Registration::transient(|provider| Folder {
            parent: provider.get::<Folder>(),
            template: provider.get::<Folder>(),
        }))
        .build();

    let folder = provider.get::<Folder>().unwrap();

    assert!(folder.parent.is_none() && folder.template.is_none());
}

/// Ein Menüeintrag, der beim Bau alle Einträge seines Menüs sammelt.
struct MenuItem {
    siblings: Vec<Rc<MenuItem>>,
}

#[test]
fn recipe_asking_all_for_own_type_gets_empty_list() {
    let provider = Provider::builder()
        .register(Registration::transient(|provider| MenuItem {
            siblings: provider.all::<MenuItem>(),
        }))
        .build();

    let items = provider.all::<MenuItem>();

    assert!(matches!(items.as_slice(), [item] if item.siblings.is_empty()));
}

#[test]
fn recipe_asking_all_for_own_type_through_get_gets_empty_list() {
    let provider = Provider::builder()
        .register(Registration::transient(|provider| MenuItem {
            siblings: provider.all::<MenuItem>(),
        }))
        .build();

    let item = provider.get::<MenuItem>().unwrap();

    assert!(item.siblings.is_empty());
}

/// Ein Kurs mit allen Studierenden, die beim Bau bekannt sind.
struct Course {
    students: Vec<Rc<Student>>,
}

/// Eine Studentin mit allen Kursen, die beim Bau bekannt sind.
struct Student {
    courses: Vec<Rc<Course>>,
}

#[test]
fn mutually_dependent_recipes_get_empty_list_from_inner_all() {
    let provider = Provider::builder()
        .register(Registration::transient(|provider| Course {
            students: provider.all::<Student>(),
        }))
        .register(Registration::transient(|provider| Student {
            courses: provider.all::<Course>(),
        }))
        .build();

    let courses = provider.all::<Course>();

    assert!(matches!(
        courses.as_slice(),
        [course] if matches!(course.students.as_slice(), [student] if student.courses.is_empty())
    ));
}

/// Ein Team mit seiner Leitung, falls es eine gibt.
struct Team {
    lead: Option<Rc<Employee>>,
}

/// Eine Mitarbeiterin mit ihrem Team, falls es eins gibt.
struct Employee {
    team: Option<Rc<Team>>,
}

#[test]
fn mutually_dependent_recipes_get_none_for_inner_query() {
    let provider = Provider::builder()
        .register(Registration::transient(|provider| Team {
            lead: provider.get::<Employee>(),
        }))
        .register(Registration::transient(|provider| Employee {
            team: provider.get::<Team>(),
        }))
        .build();

    let team = provider.get::<Team>().unwrap();

    assert!(team.lead.as_ref().unwrap().team.is_none());
}

/// Eine Verbindung zu einem Server, der nicht immer erreichbar ist.
struct Connection;

#[test]
fn recipe_runs_again_after_caught_panic() {
    let (server_up, recipe) = connection_recipe();
    let provider = Provider::builder()
        .register(Registration::transient(recipe))
        .build();
    let failed = panic::catch_unwind(AssertUnwindSafe(|| provider.get::<Connection>()));
    assert!(failed.is_err());
    server_up.set(true);

    let connection = provider.get::<Connection>();

    assert!(connection.is_some());
}

#[test]
fn singleton_recipe_runs_again_after_caught_panic() {
    let (server_up, recipe) = connection_recipe();
    let provider = Provider::builder()
        .register(Registration::singleton(recipe))
        .build();
    let failed = panic::catch_unwind(AssertUnwindSafe(|| provider.get::<Connection>()));
    assert!(failed.is_err());
    server_up.set(true);

    let connection = provider.get::<Connection>();

    assert!(connection.is_some());
}

#[test]
fn scoped_recipe_runs_again_after_caught_panic() {
    let (server_up, recipe) = connection_recipe();
    let scope = Provider::builder()
        .register(Registration::scoped(recipe))
        .build()
        .scope();
    let failed = panic::catch_unwind(AssertUnwindSafe(|| scope.get::<Connection>()));
    assert!(failed.is_err());
    server_up.set(true);

    let connection = scope.get::<Connection>();

    assert!(connection.is_some());
}

/// Liest Daten über eine Verbindung, falls es eine gibt.
struct Repository {
    connection: Option<Rc<Connection>>,
}

#[test]
fn recipe_runs_again_after_caught_panic_in_dependency() {
    let (server_up, recipe) = connection_recipe();
    let provider = Provider::builder()
        .register(Registration::transient(recipe))
        .register(Registration::transient(|provider| Repository {
            connection: provider.get::<Connection>(),
        }))
        .build();
    let failed = panic::catch_unwind(AssertUnwindSafe(|| provider.get::<Repository>()));
    assert!(failed.is_err());
    server_up.set(true);

    let repository = provider.get::<Repository>().unwrap();

    assert!(repository.connection.is_some());
}

/// Liefert den Schalter für den Server (anfangs aus) und eine Bauanleitung für `Connection`, die mit einem Panic abbricht, solange er aus ist.
fn connection_recipe() -> (Rc<Cell<bool>>, impl Fn(&Provider) -> Connection) {
    let server_up = Rc::new(Cell::new(false));
    let reachable = Rc::clone(&server_up);
    let recipe = move |_: &Provider| {
        assert!(reachable.get(), "Server nicht erreichbar");
        Connection
    };
    (server_up, recipe)
}

/// Liefert eine Bauanleitung für `Config` und den Zähler ihrer Läufe.
fn counting_recipe() -> (Rc<Cell<u32>>, impl Fn(&Provider) -> Config) {
    let calls = Rc::new(Cell::new(0_u32));
    let counter = Rc::clone(&calls);
    let recipe = move |_: &Provider| {
        counter.set(counter.get().saturating_add(1));
        Config
    };
    (calls, recipe)
}

/// Eine Rechnung mit ihrer Überschrift.
struct Invoice {
    heading: String,
}

/// Ein druckfertiges Dokument mit seinem Titel.
struct Document(String);

/// Macht aus einer Rechnung ein Dokument mit ihrer Überschrift als Titel.
impl From<Invoice> for Document {
    fn from(invoice: Invoice) -> Self {
        Self(invoice.heading)
    }
}

#[test]
fn unique_instance_twice_from_same_source_type_all_returns_first() {
    let provider = Provider::builder()
        .register(
            Registration::instance(Document::from(Invoice {
                heading: "Rechnung 1".to_owned(),
            }))
            .unique_by::<Invoice>(),
        )
        .register(
            Registration::instance(Document::from(Invoice {
                heading: "Rechnung 2".to_owned(),
            }))
            .unique_by::<Invoice>(),
        )
        .build();

    let titles: Vec<_> = provider
        .all::<Document>()
        .iter()
        .map(|document| document.0.clone())
        .collect();

    assert_eq!(titles, ["Rechnung 1"]);
}

/// Ein Lieferschein mit seiner Überschrift.
struct DeliveryNote {
    heading: String,
}

/// Macht aus einem Lieferschein ein Dokument mit seiner Überschrift als Titel.
impl From<DeliveryNote> for Document {
    fn from(note: DeliveryNote) -> Self {
        Self(note.heading)
    }
}

#[test]
fn unique_instance_from_different_source_types_all_returns_both_in_order() {
    let provider = Provider::builder()
        .register(
            Registration::instance(Document::from(Invoice {
                heading: "Rechnung 1".to_owned(),
            }))
            .unique_by::<Invoice>(),
        )
        .register(
            Registration::instance(Document::from(DeliveryNote {
                heading: "Lieferschein 1".to_owned(),
            }))
            .unique_by::<DeliveryNote>(),
        )
        .build();

    let titles: Vec<_> = provider
        .all::<Document>()
        .iter()
        .map(|document| document.0.clone())
        .collect();

    assert_eq!(titles, ["Rechnung 1", "Lieferschein 1"]);
}

#[test]
fn unique_instance_twice_from_same_source_type_get_returns_first() {
    let provider = Provider::builder()
        .register(
            Registration::instance(Document::from(Invoice {
                heading: "Rechnung 1".to_owned(),
            }))
            .unique_by::<Invoice>(),
        )
        .register(
            Registration::instance(Document::from(Invoice {
                heading: "Rechnung 2".to_owned(),
            }))
            .unique_by::<Invoice>(),
        )
        .build();

    let title = provider
        .get::<Document>()
        .map(|document| document.0.clone());

    assert_eq!(title.as_deref(), Some("Rechnung 1"));
}

/// Ein Eintrag im Kassenbuch mit seinem Text.
struct LedgerEntry(String);

/// Bucht eine Rechnung mit ihrer Überschrift als Text ins Kassenbuch.
impl From<Invoice> for LedgerEntry {
    fn from(invoice: Invoice) -> Self {
        Self(invoice.heading)
    }
}

#[test]
fn unique_instance_from_same_source_type_for_two_service_types_registers_both() {
    let provider = Provider::builder()
        .register(
            Registration::instance(Document::from(Invoice {
                heading: "Rechnung 1".to_owned(),
            }))
            .unique_by::<Invoice>(),
        )
        .register(
            Registration::instance(LedgerEntry::from(Invoice {
                heading: "Rechnung 2".to_owned(),
            }))
            .unique_by::<Invoice>(),
        )
        .build();

    let document = provider
        .get::<Document>()
        .map(|document| document.0.clone());
    let entry = provider.get::<LedgerEntry>().map(|entry| entry.0.clone());

    assert_eq!(
        (document.as_deref(), entry.as_deref()),
        (Some("Rechnung 1"), Some("Rechnung 2"))
    );
}

#[test]
fn instance_then_unique_instance_of_same_type_all_returns_both_in_order() {
    let provider = Provider::builder()
        .register(Registration::instance(Document("Aushang".to_owned())))
        .register(
            Registration::instance(Document::from(Invoice {
                heading: "Rechnung 1".to_owned(),
            }))
            .unique_by::<Invoice>(),
        )
        .build();

    let titles: Vec<_> = provider
        .all::<Document>()
        .iter()
        .map(|document| document.0.clone())
        .collect();

    assert_eq!(titles, ["Aushang", "Rechnung 1"]);
}

#[test]
fn unique_instance_from_same_source_type_for_two_service_types_all_returns_each_own() {
    let provider = Provider::builder()
        .register(
            Registration::instance(Document::from(Invoice {
                heading: "Rechnung 1".to_owned(),
            }))
            .unique_by::<Invoice>(),
        )
        .register(
            Registration::instance(LedgerEntry::from(Invoice {
                heading: "Rechnung 2".to_owned(),
            }))
            .unique_by::<Invoice>(),
        )
        .build();

    let titles: Vec<_> = provider
        .all::<Document>()
        .iter()
        .map(|document| document.0.clone())
        .collect();
    let entries: Vec<_> = provider
        .all::<LedgerEntry>()
        .iter()
        .map(|entry| entry.0.clone())
        .collect();

    assert_eq!(
        (titles, entries),
        (vec!["Rechnung 1".to_owned()], vec!["Rechnung 2".to_owned()])
    );
}

#[test]
fn mapped_registration_get_returns_converted_service() {
    let provider = Provider::builder()
        .register(
            Registration::instance(Invoice {
                heading: "Rechnung 1".to_owned(),
            })
            .map(|invoice: Rc<Invoice>| Document(invoice.heading.clone())),
        )
        .build();

    let title = provider
        .get::<Document>()
        .map(|document| document.0.clone());

    assert_eq!(title.as_deref(), Some("Rechnung 1"));
}

#[test]
fn unique_then_mapped_registration_twice_all_returns_first() {
    let provider = Provider::builder()
        .register(
            Registration::instance(Invoice {
                heading: "Rechnung 1".to_owned(),
            })
            .unique_by::<Invoice>()
            .map(|invoice: Rc<Invoice>| Document(invoice.heading.clone())),
        )
        .register(
            Registration::instance(Invoice {
                heading: "Rechnung 2".to_owned(),
            })
            .unique_by::<Invoice>()
            .map(|invoice: Rc<Invoice>| Document(invoice.heading.clone())),
        )
        .build();

    let titles: Vec<_> = provider
        .all::<Document>()
        .iter()
        .map(|document| document.0.clone())
        .collect();

    assert_eq!(titles, ["Rechnung 1"]);
}

#[test]
fn mapped_singleton_passes_same_object_to_f_on_every_get() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let provider = Provider::builder()
        .register(
            Registration::singleton(|_| Invoice {
                heading: "Rechnung 1".to_owned(),
            })
            .map({
                let seen = Rc::clone(&seen);
                move |invoice: Rc<Invoice>| {
                    seen.borrow_mut().push(Rc::clone(&invoice));
                    Document(invoice.heading.clone())
                }
            }),
        )
        .build();

    provider.get::<Document>().unwrap();
    provider.get::<Document>().unwrap();

    assert!(matches!(
        seen.borrow().as_slice(),
        [first, second] if Rc::ptr_eq(first, second)
    ));
}

#[test]
fn unique_transient_twice_from_same_source_type_all_builds_only_first() {
    let built = Rc::new(RefCell::new(Vec::new()));
    let print = |heading: &'static str| {
        let built = Rc::clone(&built);
        Registration::transient(move |_| {
            built.borrow_mut().push(heading);
            Document::from(Invoice {
                heading: heading.to_owned(),
            })
        })
        .unique_by::<Invoice>()
    };
    let provider = Provider::builder()
        .register(print("Rechnung 1"))
        .register(print("Rechnung 2"))
        .build();

    let _documents = provider.all::<Document>();

    assert_eq!(*built.borrow(), ["Rechnung 1"]);
}
