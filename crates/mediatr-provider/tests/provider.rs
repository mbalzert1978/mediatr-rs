//! Tests für den Abschnitt „Provider-Crate `mediatr-provider`“ in `TESTLIST.md`.

use std::rc::Rc;

use mediatr_provider::Provider;

struct Config;

#[test]
fn instance_returns_same_object_on_every_get() {
    let provider = Provider::builder().instance(Config).build();

    let first = provider.get::<Config>().unwrap();
    let second = provider.get::<Config>().unwrap();

    assert!(Rc::ptr_eq(&first, &second));
}
