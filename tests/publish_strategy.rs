//! Tests für den Abschnitt „Notification — Publish-Strategie“ in `TESTLIST.md`.

use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use mediatr_rs::{
    Handler, MediatorBuilder, NotificationHandler, Publisher, SendError, Sender, WeakMediator,
    WhenAllPublisher,
};

mod publish_strategy_fixtures;

use publish_strategy_fixtures::{
    ArrivalClerk, CourierPlanner, CustomsHold, DamageCheck, DeliveryDue, DispatchCenter,
    Dispatcher, DoorNotice, InspectionError, NewestFirst, ParcelArrived, ParcelInspected,
    Recipient, Scanning, ShiftCenter, Stockroom, ThoroughDamageCheck, TrackParcel, TrackingDesk,
    Unloading, Weighing,
};

#[tokio::test]
async fn custom_strategy_runs_both_handlers_in_its_own_order() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::instance(Stockroom {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(Recipient {
            journal: Rc::clone(&journal),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(
        *journal.borrow(),
        ["Empfänger benachrichtigt 7", "eingelagert 7"]
    );
}

#[tokio::test]
async fn custom_strategy_transient_handler_publishes_through_received_mediator_in_its_order() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::transient(|publisher| Dispatcher { publisher }))
        .subscribe(Handler::instance(CourierPlanner {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(DoorNotice {
            journal: Rc::clone(&journal),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(*journal.borrow(), ["Tür-Hinweis 7", "Kurier geplant 7"]);
}

#[tokio::test]
async fn custom_strategy_singleton_handler_publishes_through_received_weak_mediator_in_its_order() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::singleton(|publisher| DispatchCenter { publisher }))
        .subscribe(Handler::instance(CourierPlanner {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(DoorNotice {
            journal: Rc::clone(&journal),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(*journal.borrow(), ["Tür-Hinweis 7", "Kurier geplant 7"]);
}

#[tokio::test]
async fn custom_strategy_without_clone_mediator_and_weak_mediator_clone_reaches_same_handlers() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::singleton(
            |publisher: WeakMediator<NewestFirst>| DispatchCenter {
                publisher: publisher.clone(),
            },
        ))
        .subscribe(Handler::instance(CourierPlanner {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(DoorNotice {
            journal: Rc::clone(&journal),
        }))
        .build();

    mediator
        .clone()
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(*journal.borrow(), ["Tür-Hinweis 7", "Kurier geplant 7"]);
}

#[tokio::test]
async fn custom_strategy_without_debug_mediator_weak_mediator_and_builder_debug_names_strategy() {
    let weak_output = Rc::new(RefCell::new(String::new()));
    let seen = Rc::clone(&weak_output);
    let builder = MediatorBuilder::<NewestFirst>::default().subscribe(Handler::singleton(
        move |publisher: WeakMediator<NewestFirst>| {
            *seen.borrow_mut() = format!("{publisher:?}");
            DispatchCenter { publisher }
        },
    ));
    let builder_output = format!("{builder:?}");
    let mediator = builder.build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    let outputs = [
        format!("{mediator:?}"),
        weak_output.borrow().clone(),
        builder_output,
    ];
    assert!(
        outputs.iter().all(|output| output.contains("NewestFirst")),
        "{outputs:?}"
    );
}

#[tokio::test]
async fn custom_strategy_without_clone_builder_clone_builds_mediators_reaching_same_handlers() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let builder = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::instance(CourierPlanner {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(DoorNotice {
            journal: Rc::clone(&journal),
        }));
    let copy = builder.clone();

    builder
        .build()
        .publish(DeliveryDue { tracking: 7 })
        .await
        .unwrap();
    copy.build()
        .publish(DeliveryDue { tracking: 8 })
        .await
        .unwrap();

    assert_eq!(
        *journal.borrow(),
        [
            "Tür-Hinweis 7",
            "Kurier geplant 7",
            "Tür-Hinweis 8",
            "Kurier geplant 8"
        ]
    );
}

#[tokio::test]
async fn custom_strategy_send_returns_handler_response() {
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .register(Handler::instance(TrackingDesk))
        .build();

    let place = mediator.send(TrackParcel { tracking: 7 }).await;

    assert_eq!(place, Ok("Paket 7 liegt im Lager".to_owned()));
}

#[tokio::test]
async fn custom_strategy_singleton_handler_sends_through_received_weak_mediator() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let clerk_journal = Rc::clone(&journal);
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .register(Handler::instance(TrackingDesk))
        .subscribe(Handler::singleton(move |sender| ArrivalClerk {
            sender,
            journal: Rc::clone(&clerk_journal),
        }))
        .build();

    let published = mediator.publish(ParcelArrived { tracking: 7 }).await;

    assert_eq!(
        (published, journal.borrow().clone()),
        (Ok(()), vec!["Paket 7 liegt im Lager".to_owned()])
    );
}

#[tokio::test]
async fn custom_strategy_singleton_handler_publishes_through_weak_mediator_and_its_clone() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::singleton(
            |publisher: WeakMediator<NewestFirst>| ShiftCenter {
                night: publisher.clone(),
                day: publisher,
            },
        ))
        .subscribe(Handler::instance(CourierPlanner {
            journal: Rc::clone(&journal),
        }))
        .build();

    let published = mediator.publish(ParcelArrived { tracking: 7 }).await;

    assert_eq!(
        (published, journal.borrow().clone()),
        (
            Ok(()),
            vec!["Kurier geplant 7".to_owned(), "Kurier geplant 8".to_owned()]
        )
    );
}

#[tokio::test]
async fn custom_strategy_singleton_handler_publishing_after_mediator_dropped_gets_mediator_dropped()
{
    let handed_out = Rc::new(OnceCell::new());
    let keep = Rc::clone(&handed_out);
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::singleton(
            move |publisher: WeakMediator<NewestFirst>| {
                keep.get_or_init(|| publisher.clone());
                DispatchCenter { publisher }
            },
        ))
        .build();
    mediator
        .publish(ParcelArrived { tracking: 6 })
        .await
        .unwrap();
    let center = DispatchCenter {
        publisher: handed_out.get().unwrap().clone(),
    };
    drop(mediator);

    let result = center.handle(&ParcelArrived { tracking: 7 }).await;

    assert_eq!(result, Err(SendError::MediatorDropped));
}

#[tokio::test]
async fn concurrent_strategy_starts_second_waiting_handler_before_first_finished() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(Unloading {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(Scanning {
            journal: Rc::clone(&journal),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(
        *journal.borrow(),
        [
            "Entladen beginnt 7",
            "Scannen beginnt 7",
            "Entladen fertig 7",
            "Scannen fertig 7"
        ]
    );
}

#[tokio::test]
async fn concurrent_strategy_runs_second_handler_after_first_fails_and_returns_first_error() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(DamageCheck))
        .subscribe(Handler::instance(Weighing {
            journal: Rc::clone(&journal),
        }))
        .build();

    let published = mediator.publish(ParcelInspected { tracking: 7 }).await;

    assert_eq!(
        (published, journal.borrow().clone()),
        (
            Err(InspectionError::Damaged(7)),
            vec!["gewogen 7".to_owned()]
        )
    );
}

#[tokio::test]
async fn concurrent_strategy_runs_each_of_three_handlers_once_and_returns_ok() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(Stockroom {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(Recipient {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(Scanning {
            journal: Rc::clone(&journal),
        }))
        .build();

    let published = mediator.publish(ParcelArrived { tracking: 7 }).await;

    let mut entries = journal.borrow().clone();
    entries.sort();
    assert_eq!(
        (published, entries),
        (
            Ok(()),
            vec![
                "Empfänger benachrichtigt 7".to_owned(),
                "Scannen beginnt 7".to_owned(),
                "Scannen fertig 7".to_owned(),
                "eingelagert 7".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn concurrent_strategy_both_handlers_fail_returns_error_of_first_registered() {
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(DamageCheck))
        .subscribe(Handler::instance(CustomsHold))
        .build();

    let published = mediator.publish(ParcelInspected { tracking: 7 }).await;

    assert_eq!(published, Err(InspectionError::Damaged(7)));
}

#[tokio::test]
async fn concurrent_strategy_only_second_handler_fails_runs_first_and_returns_second_error() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(Weighing {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(DamageCheck))
        .build();

    let published = mediator.publish(ParcelInspected { tracking: 7 }).await;

    assert_eq!(
        (published, journal.borrow().clone()),
        (
            Err(InspectionError::Damaged(7)),
            vec!["gewogen 7".to_owned()]
        )
    );
}

#[tokio::test]
async fn concurrent_strategy_both_fail_first_after_await_returns_error_of_first_registered() {
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(ThoroughDamageCheck))
        .subscribe(Handler::instance(CustomsHold))
        .build();

    let published = mediator.publish(ParcelInspected { tracking: 7 }).await;

    assert_eq!(published, Err(InspectionError::Damaged(7)));
}

#[tokio::test]
async fn concurrent_strategy_equally_waiting_handlers_finish_in_registration_order() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(Unloading {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(Scanning {
            journal: Rc::clone(&journal),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    let finished: Vec<_> = journal
        .borrow()
        .iter()
        .filter(|entry| entry.contains("fertig"))
        .cloned()
        .collect();
    assert_eq!(finished, ["Entladen fertig 7", "Scannen fertig 7"]);
}
