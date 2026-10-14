//! Tests für den Abschnitt „Provider-Crate `mediatr-provider`“ in `TESTLIST.md`.

use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

use mediatr_provider::{Provider, WeakProvider};

struct Config;

struct Name(&'static str);

#[test]
fn instance_returns_same_object_on_every_get() {
    let provider = Provider::builder().instance(Config).build();

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
        .instance(Name("alt"))
        .instance(Name("neu"))
        .build();

    let name = provider.get::<Name>().unwrap();

    assert_eq!(name.0, "neu");
}

#[test]
fn all_returns_every_instance_in_registration_order() {
    let provider = Provider::builder()
        .instance(Name("alt"))
        .instance(Name("neu"))
        .build();

    let names: Vec<_> = provider.all::<Name>().iter().map(|name| name.0).collect();

    assert_eq!(names, ["alt", "neu"]);
}

#[test]
fn all_without_registration_returns_empty_list() {
    let provider = Provider::builder().instance(Name("alt")).build();

    let configs = provider.all::<Config>();

    assert!(configs.is_empty());
}

#[test]
fn transient_all_runs_recipe_on_every_call() {
    let (calls, recipe) = counting_recipe();
    let provider = Provider::builder().transient(recipe).build();

    drop(provider.all::<Config>());
    drop(provider.all::<Config>());

    assert_eq!(calls.get(), 2);
}

#[test]
fn singleton_all_returns_same_object_as_get() {
    let provider = Provider::builder().singleton(|_| Config).build();

    let single = provider.get::<Config>().unwrap();
    let all = provider.all::<Config>();

    assert!(matches!(all.as_slice(), [config] if Rc::ptr_eq(config, &single)));
}

#[test]
fn scoped_all_returns_same_object_as_get_within_scope() {
    let provider = Provider::builder().scoped(|_| Config).build();
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
    let provider = Provider::builder().transient(recipe).build();

    provider.get::<Config>().unwrap();
    provider.get::<Config>().unwrap();

    assert_eq!(calls.get(), 2);
}

#[test]
fn singleton_runs_recipe_once_on_first_get() {
    let (calls, recipe) = counting_recipe();
    let provider = Provider::builder().singleton(recipe).build();
    assert_eq!(calls.get(), 0);

    provider.get::<Config>().unwrap();
    provider.get::<Config>().unwrap();

    assert_eq!(calls.get(), 1);
}

#[test]
fn scoped_runs_recipe_once_per_scope() {
    let (calls, recipe) = counting_recipe();
    let provider = Provider::builder().scoped(recipe).build();
    let first = provider.scope();
    let second = provider.scope();

    first.get::<Config>().unwrap();
    first.get::<Config>().unwrap();
    second.get::<Config>().unwrap();

    assert_eq!(calls.get(), 2);
}

#[test]
fn singleton_returns_same_object_in_new_scope() {
    let provider = Provider::builder().singleton(|_| Config).build();

    let first = provider.get::<Config>().unwrap();
    let second = provider.scope().get::<Config>().unwrap();

    assert!(Rc::ptr_eq(&first, &second));
}

struct Greeting(String);

#[test]
fn recipe_gets_other_service_from_provider() {
    let provider = Provider::builder()
        .instance(Name("Welt"))
        .transient(|provider| Greeting(format!("Hallo {}", provider.get::<Name>().unwrap().0)))
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
        .scoped(move |_| {
            opened.set(opened.get().saturating_add(1));
            Session(opened.get())
        })
        .singleton(|provider| AuditLog {
            session: provider.get::<Session>().unwrap(),
        })
        .build();

    let log = provider.scope().get::<AuditLog>().unwrap();
    let provider_session = provider.get::<Session>().unwrap();

    assert_eq!(log.session.0, provider_session.0);
}

#[test]
fn two_scoped_registrations_of_same_type_build_one_object_each() {
    let scope = Provider::builder()
        .scoped(|_| Name("haupt"))
        .scoped(|_| Name("ersatz"))
        .build()
        .scope();

    let names: Vec<_> = scope.all::<Name>().iter().map(|name| name.0).collect();

    assert_eq!(names, ["haupt", "ersatz"]);
}

#[test]
fn cloned_scope_returns_same_scoped_service() {
    let (calls, recipe) = counting_recipe();
    let scope = Provider::builder().scoped(recipe).build().scope();
    let clone = scope.clone();

    scope.get::<Config>().unwrap();
    clone.get::<Config>().unwrap();

    assert_eq!(calls.get(), 1);
}

#[test]
fn upgraded_weak_provider_returns_same_singleton() {
    let provider = Provider::builder().singleton(|_| Config).build();
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
        .singleton(move |provider| Scheduler {
            _provider: provider.downgrade(),
            _alive: Alive(Rc::clone(&flag)),
        })
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
    let scope = Provider::builder().scoped(|_| Config).build().scope();
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
        .scoped(move |scope| UnitOfWork {
            _scope: scope.downgrade(),
            _alive: Alive(Rc::clone(&flag)),
        })
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
        .singleton(move |provider| Scheduler {
            _provider: provider.downgrade(),
            _alive: Alive(Rc::clone(&running_flag)),
        })
        .scoped(move |provider| Report {
            _scheduler: provider.get::<Scheduler>().unwrap(),
            _alive: Alive(Rc::clone(&open_flag)),
        })
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
    let provider = Provider::builder().transient(settings_recipe).build();

    let settings = provider.get::<Settings>().unwrap();

    assert!(settings.parent.is_none());
}

#[test]
fn first_of_two_recipes_asking_for_own_type_through_all_gets_none() {
    let provider = Provider::builder()
        .transient(settings_recipe)
        .transient(|_| Settings { parent: None })
        .build();

    let settings = provider.all::<Settings>();

    assert!(matches!(settings.as_slice(), [first, _] if first.parent.is_none()));
}

#[test]
fn singleton_recipe_asking_for_own_type_gets_none() {
    let provider = Provider::builder().singleton(settings_recipe).build();

    let settings = provider.get::<Settings>().unwrap();

    assert!(settings.parent.is_none());
}

#[test]
fn scoped_recipe_asking_for_own_type_gets_none() {
    let provider = Provider::builder().scoped(settings_recipe).build();

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
        .transient(|provider| Folder {
            parent: provider.get::<Folder>(),
            template: provider.get::<Folder>(),
        })
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
        .transient(|provider| MenuItem {
            siblings: provider.all::<MenuItem>(),
        })
        .build();

    let items = provider.all::<MenuItem>();

    assert!(matches!(items.as_slice(), [item] if item.siblings.is_empty()));
}

#[test]
fn recipe_asking_all_for_own_type_through_get_gets_empty_list() {
    let provider = Provider::builder()
        .transient(|provider| MenuItem {
            siblings: provider.all::<MenuItem>(),
        })
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
        .transient(|provider| Course {
            students: provider.all::<Student>(),
        })
        .transient(|provider| Student {
            courses: provider.all::<Course>(),
        })
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
        .transient(|provider| Team {
            lead: provider.get::<Employee>(),
        })
        .transient(|provider| Employee {
            team: provider.get::<Team>(),
        })
        .build();

    let team = provider.get::<Team>().unwrap();

    assert!(team.lead.as_ref().unwrap().team.is_none());
}

/// Eine Verbindung zu einem Server, der nicht immer erreichbar ist.
struct Connection;

#[test]
fn recipe_runs_again_after_caught_panic() {
    let (server_up, recipe) = connection_recipe();
    let provider = Provider::builder().transient(recipe).build();
    let failed = panic::catch_unwind(AssertUnwindSafe(|| provider.get::<Connection>()));
    assert!(failed.is_err());
    server_up.set(true);

    let connection = provider.get::<Connection>();

    assert!(connection.is_some());
}

#[test]
fn singleton_recipe_runs_again_after_caught_panic() {
    let (server_up, recipe) = connection_recipe();
    let provider = Provider::builder().singleton(recipe).build();
    let failed = panic::catch_unwind(AssertUnwindSafe(|| provider.get::<Connection>()));
    assert!(failed.is_err());
    server_up.set(true);

    let connection = provider.get::<Connection>();

    assert!(connection.is_some());
}

#[test]
fn scoped_recipe_runs_again_after_caught_panic() {
    let (server_up, recipe) = connection_recipe();
    let scope = Provider::builder().scoped(recipe).build().scope();
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
        .transient(recipe)
        .transient(|provider| Repository {
            connection: provider.get::<Connection>(),
        })
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
