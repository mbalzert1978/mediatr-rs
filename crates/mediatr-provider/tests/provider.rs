//! Tests für den Abschnitt „Provider-Crate `mediatr-provider`“ in `TESTLIST.md`.

use std::cell::Cell;
use std::rc::Rc;

use mediatr_provider::Provider;

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
fn cloned_scope_returns_same_scoped_service() {
    let (calls, recipe) = counting_recipe();
    let scope = Provider::builder().scoped(recipe).build().scope();
    let clone = scope.clone();

    scope.get::<Config>().unwrap();
    clone.get::<Config>().unwrap();

    assert_eq!(calls.get(), 1);
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
