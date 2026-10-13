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

/// Liefert eine Bauanleitung für `Config` und den Zähler ihrer Läufe.
fn counting_recipe() -> (Rc<Cell<u32>>, impl Fn() -> Config) {
    let calls = Rc::new(Cell::new(0_u32));
    let counter = Rc::clone(&calls);
    let recipe = move || {
        counter.set(counter.get().saturating_add(1));
        Config
    };
    (calls, recipe)
}

#[test]
fn singleton_returns_same_object_in_new_scope() {
    let provider = Provider::builder().singleton(|| Config).build();

    let first = provider.get::<Config>().unwrap();
    let second = provider.scope().get::<Config>().unwrap();

    assert!(Rc::ptr_eq(&first, &second));
}
