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
    let calls = Rc::new(Cell::new(0_u32));
    let counter = Rc::clone(&calls);
    let provider = Provider::builder()
        .transient(move || {
            counter.set(counter.get().saturating_add(1));
            Config
        })
        .build();

    provider.get::<Config>().unwrap();
    provider.get::<Config>().unwrap();

    assert_eq!(calls.get(), 2);
}

#[test]
fn singleton_runs_recipe_once_on_first_get() {
    let calls = Rc::new(Cell::new(0_u32));
    let counter = Rc::clone(&calls);
    let provider = Provider::builder()
        .singleton(move || {
            counter.set(counter.get().saturating_add(1));
            Config
        })
        .build();
    assert_eq!(calls.get(), 0);

    provider.get::<Config>().unwrap();
    provider.get::<Config>().unwrap();

    assert_eq!(calls.get(), 1);
}
