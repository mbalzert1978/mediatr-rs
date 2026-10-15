//! Tests für den Abschnitt „Notification — Notification-Publisher“ in `TESTLIST.md`.

use std::cell::OnceCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError};

use mediatr_rs::{
    Handler, MediatorBuilder, NotificationHandler, Publisher, SendError, Sender, WeakMediator,
    WhenAllPublisher,
};

mod common;
mod notification_publishers_fixtures;

use common::Journal;
use notification_publishers_fixtures::{
    ArrivalClerk, CourierPlanner, CustomsHold, DamageCheck, DeliveryDue, DispatchCenter,
    Dispatcher, DoorNotice, InspectionError, NewestFirst, ParcelArrived, ParcelInspected,
    Recipient, Scanning, ShiftCenter, Stockroom, ThoroughDamageCheck, TrackParcel, TrackingDesk,
    Unloading, Weighing,
};

#[tokio::test]
async fn custom_publisher_runs_both_handlers_in_its_own_order() {
    let journal = Journal::default();
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::instance(Stockroom {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(Recipient {
            journal: journal.clone(),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(
        journal.entries(),
        ["Empfänger benachrichtigt 7", "eingelagert 7"]
    );
}

#[tokio::test]
async fn custom_publisher_transient_handler_publishes_through_received_mediator_in_its_order() {
    let journal = Journal::default();
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::transient(|publisher| Dispatcher { publisher }))
        .subscribe(Handler::instance(CourierPlanner {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(DoorNotice {
            journal: journal.clone(),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(journal.entries(), ["Tür-Hinweis 7", "Kurier geplant 7"]);
}

#[tokio::test]
async fn custom_publisher_singleton_handler_publishes_through_received_weak_mediator_in_its_order()
{
    let journal = Journal::default();
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::singleton(|publisher| DispatchCenter { publisher }))
        .subscribe(Handler::instance(CourierPlanner {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(DoorNotice {
            journal: journal.clone(),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(journal.entries(), ["Tür-Hinweis 7", "Kurier geplant 7"]);
}

#[tokio::test]
async fn custom_publisher_without_clone_mediator_and_weak_mediator_clone_reaches_same_handlers() {
    let journal = Journal::default();
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::singleton(
            |publisher: WeakMediator<NewestFirst>| DispatchCenter {
                publisher: publisher.clone(),
            },
        ))
        .subscribe(Handler::instance(CourierPlanner {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(DoorNotice {
            journal: journal.clone(),
        }))
        .build();

    mediator
        .clone()
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(journal.entries(), ["Tür-Hinweis 7", "Kurier geplant 7"]);
}

#[tokio::test]
async fn custom_publisher_without_debug_mediator_weak_mediator_and_builder_debug_names_publisher() {
    let weak_output = Arc::new(Mutex::new(String::new()));
    let seen = Arc::clone(&weak_output);
    let builder = MediatorBuilder::<NewestFirst>::default().subscribe(Handler::singleton(
        move |publisher: WeakMediator<NewestFirst>| {
            *seen.lock().unwrap_or_else(PoisonError::into_inner) = format!("{publisher:?}");
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
        weak_output
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone(),
        builder_output,
    ];
    assert!(
        outputs.iter().all(|output| output.contains("NewestFirst")),
        "{outputs:?}"
    );
}

#[tokio::test]
async fn custom_publisher_without_clone_builder_clone_builds_mediators_reaching_same_handlers() {
    let journal = Journal::default();
    let builder = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::instance(CourierPlanner {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(DoorNotice {
            journal: journal.clone(),
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
        journal.entries(),
        [
            "Tür-Hinweis 7",
            "Kurier geplant 7",
            "Tür-Hinweis 8",
            "Kurier geplant 8"
        ]
    );
}

#[tokio::test]
async fn custom_publisher_send_returns_handler_response() {
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .register(Handler::instance(TrackingDesk))
        .build();

    let place = mediator.send(TrackParcel { tracking: 7 }).await;

    assert_eq!(place, Ok("Paket 7 liegt im Lager".to_owned()));
}

#[tokio::test]
async fn custom_publisher_singleton_handler_sends_through_received_weak_mediator() {
    let journal = Journal::default();
    let clerk_journal = journal.clone();
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .register(Handler::instance(TrackingDesk))
        .subscribe(Handler::singleton(move |sender| ArrivalClerk {
            sender,
            journal: clerk_journal.clone(),
        }))
        .build();

    let published = mediator.publish(ParcelArrived { tracking: 7 }).await;

    assert_eq!(
        (published, journal.entries()),
        (Ok(()), vec!["Paket 7 liegt im Lager".to_owned()])
    );
}

#[tokio::test]
async fn custom_publisher_singleton_handler_publishes_through_weak_mediator_and_its_clone() {
    let journal = Journal::default();
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::singleton(
            |publisher: WeakMediator<NewestFirst>| ShiftCenter {
                night: publisher.clone(),
                day: publisher,
            },
        ))
        .subscribe(Handler::instance(CourierPlanner {
            journal: journal.clone(),
        }))
        .build();

    let published = mediator.publish(ParcelArrived { tracking: 7 }).await;

    assert_eq!(
        (published, journal.entries()),
        (
            Ok(()),
            vec!["Kurier geplant 7".to_owned(), "Kurier geplant 8".to_owned()]
        )
    );
}

#[tokio::test]
async fn custom_publisher_singleton_handler_publishing_after_mediator_dropped_gets_mediator_dropped()
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
async fn when_all_publisher_starts_second_waiting_handler_before_first_finished() {
    let journal = Journal::default();
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(Unloading {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(Scanning {
            journal: journal.clone(),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    let entries = journal.entries();
    let position = |entry: &str| entries.iter().position(|logged| logged == entry);
    assert!(matches!(
        (position("Scannen beginnt 7"), position("Entladen fertig 7")),
        (Some(scanning), Some(unloaded)) if scanning < unloaded
    ));
}

#[tokio::test]
async fn when_all_publisher_runs_second_handler_after_first_fails_and_returns_first_error() {
    let journal = Journal::default();
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(DamageCheck))
        .subscribe(Handler::instance(Weighing {
            journal: journal.clone(),
        }))
        .build();

    let published = mediator.publish(ParcelInspected { tracking: 7 }).await;

    assert_eq!(
        (published, journal.entries()),
        (
            Err(InspectionError::Damaged(7)),
            vec!["gewogen 7".to_owned()]
        )
    );
}

#[tokio::test]
async fn when_all_publisher_runs_each_of_three_handlers_once_and_returns_ok() {
    let journal = Journal::default();
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(Stockroom {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(Recipient {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(Scanning {
            journal: journal.clone(),
        }))
        .build();

    let published = mediator.publish(ParcelArrived { tracking: 7 }).await;

    let mut entries = journal.entries();
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
async fn when_all_publisher_both_handlers_fail_returns_error_of_first_registered() {
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(DamageCheck))
        .subscribe(Handler::instance(CustomsHold))
        .build();

    let published = mediator.publish(ParcelInspected { tracking: 7 }).await;

    assert_eq!(published, Err(InspectionError::Damaged(7)));
}

#[tokio::test]
async fn when_all_publisher_only_second_handler_fails_runs_first_and_returns_second_error() {
    let journal = Journal::default();
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(Weighing {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(DamageCheck))
        .build();

    let published = mediator.publish(ParcelInspected { tracking: 7 }).await;

    assert_eq!(
        (published, journal.entries()),
        (
            Err(InspectionError::Damaged(7)),
            vec!["gewogen 7".to_owned()]
        )
    );
}

#[tokio::test]
async fn when_all_publisher_both_fail_first_after_await_returns_error_of_first_registered() {
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(ThoroughDamageCheck))
        .subscribe(Handler::instance(CustomsHold))
        .build();

    let published = mediator.publish(ParcelInspected { tracking: 7 }).await;

    assert_eq!(published, Err(InspectionError::Damaged(7)));
}

#[tokio::test]
async fn when_all_publisher_equally_waiting_handlers_finish_in_registration_order() {
    let journal = Journal::default();
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .subscribe(Handler::instance(Unloading {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(Scanning {
            journal: journal.clone(),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    let finished: Vec<_> = journal
        .entries()
        .iter()
        .filter(|entry| entry.contains("fertig"))
        .cloned()
        .collect();
    assert_eq!(finished, ["Entladen fertig 7", "Scannen fertig 7"]);
}
