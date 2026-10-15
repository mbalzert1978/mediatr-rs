//! Tests für den Abschnitt „Provider-Crate `mediatr-provider`“ in `TESTLIST.md`.

use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex, PoisonError};

use mediatr_provider::{GetError, Provider, Registration};

mod provider_fixtures;

use provider_fixtures::{
    Alive, AuditLog, Config, Connection, Course, DeliveryNote, Document, Employee, Folder,
    Greeting, Invoice, LedgerEntry, MenuItem, Name, Printer, Report, Repository, Scheduler,
    Session, Settings, Student, Team, UnitOfWork, connection_recipe, counting_recipe, gated_recipe,
    settings_recipe,
};

#[test]
fn instance_returns_same_object_on_every_get() {
    let provider = Provider::builder()
        .register(Registration::instance(Config))
        .build();

    let first = provider.get::<Config>().unwrap();
    let second = provider.get::<Config>().unwrap();

    assert!(Arc::ptr_eq(&first, &second));
}

#[test]
fn get_without_registration_returns_no_service() {
    let provider = Provider::builder().build();

    assert!(matches!(provider.get::<Config>(), Err(GetError::NoService)));
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

    let names: Vec<_> = provider
        .all::<Name>()
        .unwrap()
        .iter()
        .map(|name| name.0)
        .collect();

    assert_eq!(names, ["alt", "neu"]);
}

#[test]
fn all_without_registration_returns_empty_list() {
    let provider = Provider::builder()
        .register(Registration::instance(Name("alt")))
        .build();

    let configs = provider.all::<Config>();

    assert!(matches!(configs, Ok(list) if list.is_empty()));
}

#[test]
fn transient_all_runs_recipe_on_every_call() {
    let (calls, recipe) = counting_recipe();
    let provider = Provider::builder()
        .register(Registration::transient(recipe))
        .build();

    drop(provider.all::<Config>());
    drop(provider.all::<Config>());

    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn singleton_all_returns_same_object_as_get() {
    let provider = Provider::builder()
        .register(Registration::singleton(|_| Config))
        .build();

    let single = provider.get::<Config>().unwrap();
    let all = provider.all::<Config>().unwrap();

    assert!(matches!(all.as_slice(), [config] if Arc::ptr_eq(config, &single)));
}

#[test]
fn scoped_all_returns_same_object_as_get_within_scope() {
    let provider = Provider::builder()
        .register(Registration::scoped(|_| Config))
        .build();
    let first = provider.scope();
    let second = provider.scope();

    let single = first.get::<Config>().unwrap();
    let same_scope = first.all::<Config>().unwrap();
    let other_scope = second.all::<Config>().unwrap();

    assert!(matches!(same_scope.as_slice(), [config] if Arc::ptr_eq(config, &single)));
    assert!(matches!(other_scope.as_slice(), [config] if !Arc::ptr_eq(config, &single)));
}

#[test]
fn transient_runs_recipe_on_every_get() {
    let (calls, recipe) = counting_recipe();
    let provider = Provider::builder()
        .register(Registration::transient(recipe))
        .build();

    provider.get::<Config>().unwrap();
    provider.get::<Config>().unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn singleton_runs_recipe_once_on_first_get() {
    let (calls, recipe) = counting_recipe();
    let provider = Provider::builder()
        .register(Registration::singleton(recipe))
        .build();
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    provider.get::<Config>().unwrap();
    provider.get::<Config>().unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
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

    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn singleton_returns_same_object_in_new_scope() {
    let provider = Provider::builder()
        .register(Registration::singleton(|_| Config))
        .build();

    let first = provider.get::<Config>().unwrap();
    let second = provider.scope().get::<Config>().unwrap();

    assert!(Arc::ptr_eq(&first, &second));
}

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

#[test]
fn two_scoped_registrations_of_same_type_build_one_object_each() {
    let scope = Provider::builder()
        .register(Registration::scoped(|_| Name("haupt")))
        .register(Registration::scoped(|_| Name("ersatz")))
        .build()
        .scope();

    let names: Vec<_> = scope
        .all::<Name>()
        .unwrap()
        .iter()
        .map(|name| name.0)
        .collect();

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

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn upgraded_weak_provider_returns_same_singleton() {
    let provider = Provider::builder()
        .register(Registration::singleton(|_| Config))
        .build();
    let weak = provider.downgrade();

    let upgraded = weak.upgrade().unwrap();

    assert!(Arc::ptr_eq(
        &provider.get::<Config>().unwrap(),
        &upgraded.get::<Config>().unwrap()
    ));
}

#[test]
fn both_instances_of_same_type_are_released_with_provider() {
    let office = Arc::new(AtomicBool::new(true));
    let lobby = Arc::new(AtomicBool::new(true));
    let provider = Provider::builder()
        .register(Registration::instance(Printer {
            _alive: Alive(Arc::clone(&office)),
        }))
        .register(Registration::instance(Printer {
            _alive: Alive(Arc::clone(&lobby)),
        }))
        .build();

    drop(provider);

    assert!(!office.load(Ordering::SeqCst) && !lobby.load(Ordering::SeqCst));
}

#[test]
fn singleton_holding_weak_provider_is_released_with_provider() {
    let running = Arc::new(AtomicBool::new(true));
    let flag = Arc::clone(&running);
    let provider = Provider::builder()
        .register(Registration::singleton(move |provider| Scheduler {
            _provider: provider.downgrade(),
            _alive: Alive(Arc::clone(&flag)),
        }))
        .build();
    provider.get::<Scheduler>().unwrap();

    drop(provider);

    assert!(!running.load(Ordering::SeqCst));
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

    assert!(Arc::ptr_eq(
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

#[test]
fn scoped_holding_weak_scope_is_released_with_scope() {
    let open = Arc::new(AtomicBool::new(true));
    let flag = Arc::clone(&open);
    let provider = Provider::builder()
        .register(Registration::scoped(move |scope| UnitOfWork {
            _scope: scope.downgrade(),
            _alive: Alive(Arc::clone(&flag)),
        }))
        .build();
    let scope = provider.scope();
    scope.get::<UnitOfWork>().unwrap();

    drop(scope);

    assert!(!open.load(Ordering::SeqCst));
}

#[test]
fn recipe_asking_for_own_type_gets_cycle() {
    let provider = Provider::builder()
        .register(Registration::transient(settings_recipe))
        .build();

    let settings = provider.get::<Settings>().unwrap();

    assert!(matches!(settings.parent, Err(GetError::Cycle)));
}

#[test]
fn first_of_two_recipes_asking_for_own_type_through_all_gets_nothing() {
    let provider = Provider::builder()
        .register(Registration::transient(settings_recipe))
        .register(Registration::transient(|_| Settings {
            parent: Err(GetError::NoService),
        }))
        .build();

    let settings = provider.all::<Settings>().unwrap();

    assert!(matches!(settings.as_slice(), [first, _] if first.parent.is_err()));
}

#[test]
fn singleton_recipe_asking_for_own_type_gets_nothing() {
    let provider = Provider::builder()
        .register(Registration::singleton(settings_recipe))
        .build();

    let settings = provider.get::<Settings>().unwrap();

    assert!(settings.parent.is_err());
}

#[test]
fn scoped_recipe_asking_for_own_type_in_scope_gets_nothing() {
    let provider = Provider::builder()
        .register(Registration::scoped(settings_recipe))
        .build();

    let settings = provider.scope().get::<Settings>().unwrap();

    assert!(settings.parent.is_err());
}

#[test]
fn recipe_asking_twice_for_own_type_gets_nothing_both_times() {
    let provider = Provider::builder()
        .register(Registration::transient(|provider| Folder {
            parent: provider.get::<Folder>().ok(),
            template: provider.get::<Folder>().ok(),
        }))
        .build();

    let folder = provider.get::<Folder>().unwrap();

    assert!(folder.parent.is_none() && folder.template.is_none());
}

#[test]
fn recipe_asking_all_for_own_type_gets_cycle() {
    let provider = Provider::builder()
        .register(Registration::transient(|provider| MenuItem {
            siblings: provider.all::<MenuItem>(),
        }))
        .build();

    let items = provider.all::<MenuItem>();

    assert!(
        matches!(items.as_deref(), Ok([item]) if matches!(item.siblings, Err(GetError::Cycle)))
    );
}

#[test]
fn recipe_asking_all_for_own_type_through_get_gets_nothing() {
    let provider = Provider::builder()
        .register(Registration::transient(|provider| MenuItem {
            siblings: provider.all::<MenuItem>(),
        }))
        .build();

    let item = provider.get::<MenuItem>().unwrap();

    assert!(item.siblings.is_err());
}

#[test]
fn mutually_dependent_recipes_get_nothing_from_inner_all() {
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
        courses.as_deref(),
        Ok([course]) if matches!(course.students.as_deref(), Ok([student]) if student.courses.is_err())
    ));
}

#[test]
fn mutually_dependent_recipes_get_nothing_for_inner_query() {
    let provider = Provider::builder()
        .register(Registration::transient(|provider| Team {
            lead: provider.get::<Employee>().ok(),
        }))
        .register(Registration::transient(|provider| Employee {
            team: provider.get::<Team>().ok(),
        }))
        .build();

    let team = provider.get::<Team>().unwrap();

    assert!(team.lead.as_ref().unwrap().team.is_none());
}

#[test]
fn recipe_runs_again_after_caught_panic() {
    let (server_up, recipe) = connection_recipe();
    let provider = Provider::builder()
        .register(Registration::transient(recipe))
        .build();
    let failed = panic::catch_unwind(AssertUnwindSafe(|| provider.get::<Connection>()));
    assert!(failed.is_err());
    server_up.store(true, Ordering::SeqCst);

    let connection = provider.get::<Connection>();

    assert!(connection.is_ok());
}

#[test]
fn singleton_recipe_runs_again_after_caught_panic() {
    let (server_up, recipe) = connection_recipe();
    let provider = Provider::builder()
        .register(Registration::singleton(recipe))
        .build();
    let failed = panic::catch_unwind(AssertUnwindSafe(|| provider.get::<Connection>()));
    assert!(failed.is_err());
    server_up.store(true, Ordering::SeqCst);

    let connection = provider.get::<Connection>();

    assert!(connection.is_ok());
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
    server_up.store(true, Ordering::SeqCst);

    let connection = scope.get::<Connection>();

    assert!(connection.is_ok());
}

#[test]
fn recipe_runs_again_after_caught_panic_in_dependency() {
    let (server_up, recipe) = connection_recipe();
    let provider = Provider::builder()
        .register(Registration::transient(recipe))
        .register(Registration::transient(|provider| Repository {
            connection: provider.get::<Connection>().ok(),
        }))
        .build();
    let failed = panic::catch_unwind(AssertUnwindSafe(|| provider.get::<Repository>()));
    assert!(failed.is_err());
    server_up.store(true, Ordering::SeqCst);

    let repository = provider.get::<Repository>().unwrap();

    assert!(repository.connection.is_some());
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
        .unwrap()
        .iter()
        .map(|document| document.0.clone())
        .collect();

    assert_eq!(titles, ["Rechnung 1"]);
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
        .unwrap()
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

    assert_eq!(title.as_deref(), Ok("Rechnung 1"));
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
        (Ok("Rechnung 1"), Ok("Rechnung 2"))
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
        .unwrap()
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
        .unwrap()
        .iter()
        .map(|document| document.0.clone())
        .collect();
    let entries: Vec<_> = provider
        .all::<LedgerEntry>()
        .unwrap()
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
            .map(|invoice: Arc<Invoice>| Document(invoice.heading.clone())),
        )
        .build();

    let title = provider
        .get::<Document>()
        .map(|document| document.0.clone());

    assert_eq!(title.as_deref(), Ok("Rechnung 1"));
}

#[test]
fn unique_then_mapped_registration_twice_all_returns_first() {
    let provider = Provider::builder()
        .register(
            Registration::instance(Invoice {
                heading: "Rechnung 1".to_owned(),
            })
            .unique_by::<Invoice>()
            .map(|invoice: Arc<Invoice>| Document(invoice.heading.clone())),
        )
        .register(
            Registration::instance(Invoice {
                heading: "Rechnung 2".to_owned(),
            })
            .unique_by::<Invoice>()
            .map(|invoice: Arc<Invoice>| Document(invoice.heading.clone())),
        )
        .build();

    let titles: Vec<_> = provider
        .all::<Document>()
        .unwrap()
        .iter()
        .map(|document| document.0.clone())
        .collect();

    assert_eq!(titles, ["Rechnung 1"]);
}

#[test]
fn mapped_singleton_passes_same_object_to_f_on_every_get() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let provider = Provider::builder()
        .register(
            Registration::singleton(|_| Invoice {
                heading: "Rechnung 1".to_owned(),
            })
            .map({
                let seen = Arc::clone(&seen);
                move |invoice: Arc<Invoice>| {
                    seen.lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .push(Arc::clone(&invoice));
                    Document(invoice.heading.clone())
                }
            }),
        )
        .build();

    provider.get::<Document>().unwrap();
    provider.get::<Document>().unwrap();

    assert!(matches!(
        seen.lock().unwrap_or_else(PoisonError::into_inner).as_slice(),
        [first, second] if Arc::ptr_eq(first, second)
    ));
}

#[test]
fn unique_transient_twice_from_same_source_type_all_builds_only_first() {
    let built = Arc::new(Mutex::new(Vec::new()));
    let print = |heading: &'static str| {
        let built = Arc::clone(&built);
        Registration::transient(move |_| {
            built
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(heading);
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

    let _documents = provider.all::<Document>().unwrap();

    assert_eq!(
        *built.lock().unwrap_or_else(PoisonError::into_inner),
        ["Rechnung 1"]
    );
}

#[test]
fn registration_debug_names_service_type() {
    let registration = Registration::instance(Invoice {
        heading: "Rechnung 1".to_owned(),
    });

    let output = format!("{registration:?}");

    assert!(output.contains("Invoice"), "{output}");
}

#[test]
fn provider_moved_to_other_thread_returns_registered_service() {
    let provider = Provider::builder()
        .register(Registration::instance(Name("Ada")))
        .build();

    let name = std::thread::spawn(move || provider.get::<Name>().map(|name| name.0))
        .join()
        .unwrap();

    assert_eq!(name, Ok("Ada"));
}

#[test]
fn second_thread_gets_transient_service_while_first_thread_builds_it() {
    let (gate, recipe) = gated_recipe();
    let provider = Provider::builder()
        .register(Registration::transient(recipe))
        .build();

    let second = std::thread::scope(|scope| {
        let first = scope.spawn(|| provider.get::<Config>());
        gate.wait_started();
        let second = scope.spawn(|| provider.get::<Config>()).join();
        gate.release();
        first.join().unwrap().unwrap();
        second.unwrap()
    });

    assert!(second.is_ok());
}

#[test]
fn two_threads_get_same_singleton_built_once() {
    let (calls, recipe) = counting_recipe();
    let provider = Provider::builder()
        .register(Registration::singleton(recipe))
        .build();
    let start = Barrier::new(2);

    let (first, second) = std::thread::scope(|scope| {
        let get = || {
            start.wait();
            provider.get::<Config>().unwrap()
        };
        let first = scope.spawn(get);
        let second = scope.spawn(get);
        (first.join().unwrap(), second.join().unwrap())
    });

    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn two_threads_get_same_scoped_service_in_one_scope() {
    let both_building = Arc::new(Barrier::new(2));
    let provider = Provider::builder()
        .register(Registration::scoped(move |_| {
            both_building.wait();
            Session
        }))
        .build();
    let scope = provider.scope();

    let (first, second) = std::thread::scope(|threads| {
        let get = || scope.get::<Session>().unwrap();
        let first = threads.spawn(get);
        let second = threads.spawn(get);
        (first.join().unwrap(), second.join().unwrap())
    });

    assert!(Arc::ptr_eq(&first, &second));
}

#[test]
fn scoped_service_from_provider_itself_gets_scoped_from_root() {
    let provider = Provider::builder()
        .register(Registration::scoped(|_| Session))
        .build();

    let session = provider.get::<Session>();

    assert!(matches!(session, Err(GetError::ScopedFromRoot)));
}

#[test]
fn singleton_recipe_asking_for_scoped_service_gets_scoped_from_root() {
    let provider = Provider::builder()
        .register(Registration::scoped(|_| Session))
        .register(Registration::singleton(|provider| AuditLog {
            session: provider.get::<Session>(),
        }))
        .build();

    let log = provider.scope().get::<AuditLog>().unwrap();

    assert!(matches!(log.session, Err(GetError::ScopedFromRoot)));
}

#[test]
fn transient_recipe_in_scope_gets_scoped_service_of_scope() {
    let provider = Provider::builder()
        .register(Registration::scoped(|_| Session))
        .register(Registration::transient(|provider| Report {
            session: provider.get::<Session>(),
        }))
        .build();
    let scope = provider.scope();

    let report = scope.get::<Report>().unwrap();
    let session = scope.get::<Session>().unwrap();

    assert!(matches!(&report.session, Ok(own) if Arc::ptr_eq(own, &session)));
}
