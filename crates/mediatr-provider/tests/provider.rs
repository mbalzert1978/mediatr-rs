//! Tests für den Abschnitt „Provider-Crate `mediatr-provider`“ in `TESTLIST.md`.

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
