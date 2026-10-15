//! Tests für den Abschnitt „Stream-Request — Stream-Behaviors“ in `TESTLIST.md`.

use futures_util::StreamExt;
use mediatr_rs::{Handler, Mediator, Sender, StreamSender};

mod common;
mod stream_behaviors_fixtures;

use common::Journal;
use stream_behaviors_fixtures::{
    CheckIn, CheckInDesk, ClosedDesk, Concierge, DepartureBoard, DoorLog, FrontOffice, Greeter,
    ListArrivals, ListDepartures, NightPorter, bellboy, concierge,
    concierge_and_night_porter_around, concierge_bellboy_and_night_porter_around, night_porter,
};

#[tokio::test]
async fn two_open_stream_behaviors_wrap_handler_first_registered_outside_items_unchanged() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_stream_behavior(Concierge {
            journal: journal.clone(),
        })
        .open_stream_behavior(NightPorter {
            journal: journal.clone(),
        })
        .register_stream(Handler::instance(FrontOffice {
            guests: vec!["Anna".to_owned(), "Ben".to_owned()],
            journal: journal.clone(),
        }))
        .build();

    let arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 5 })
        .collect()
        .await;

    assert_eq!(
        (arrivals, journal.entries()),
        (
            vec![
                Ok("Anna am Tag 5".to_owned()),
                Ok("Ben am Tag 5".to_owned())
            ],
            concierge_and_night_porter_around("Anreisen für Tag 5 nachgeschlagen")
        )
    );
}

#[tokio::test]
async fn two_stream_behaviors_for_one_type_wrap_handler_in_registration_order_items_unchanged() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .stream_behavior::<ListArrivals>(Concierge {
            journal: journal.clone(),
        })
        .stream_behavior::<ListArrivals>(NightPorter {
            journal: journal.clone(),
        })
        .register_stream(Handler::instance(FrontOffice {
            guests: vec!["Clara".to_owned()],
            journal: journal.clone(),
        }))
        .build();

    let arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 2 })
        .collect()
        .await;

    assert_eq!(
        (arrivals, journal.entries()),
        (
            vec![Ok("Clara am Tag 2".to_owned())],
            concierge_and_night_porter_around("Anreisen für Tag 2 nachgeschlagen")
        )
    );
}

#[tokio::test]
async fn stream_behavior_for_one_type_joins_general_ones_in_registration_order_other_type_gets_only_general()
 {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_stream_behavior(concierge(&journal))
        .stream_behavior::<ListArrivals>(bellboy(&journal))
        .open_stream_behavior(night_porter(&journal))
        .register_stream(Handler::instance(FrontOffice {
            guests: Vec::new(),
            journal: journal.clone(),
        }))
        .register_stream(Handler::instance(DepartureBoard {
            journal: journal.clone(),
        }))
        .build();

    let _arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 3 })
        .collect()
        .await;
    let arrivals_journal = journal.entries();
    let _departures: Vec<_> = mediator
        .create_stream(ListDepartures { day: 3 })
        .collect()
        .await;

    let departures_journal = journal.entries().split_off(arrivals_journal.len());

    assert_eq!(
        (arrivals_journal, departures_journal),
        (
            concierge_bellboy_and_night_porter_around("Anreisen für Tag 3 nachgeschlagen"),
            concierge_and_night_porter_around("Abreisen für Tag 3 nachgeschlagen")
        )
    );
}

#[tokio::test]
async fn stream_behavior_adding_one_item_before_and_after_three_handler_items_yields_five_in_order()
{
    let mediator = Mediator::builder()
        .stream_behavior::<ListArrivals>(Greeter)
        .register_stream(Handler::instance(FrontOffice {
            guests: vec!["Anna".to_owned(), "Ben".to_owned(), "Clara".to_owned()],
            journal: Journal::default(),
        }))
        .build();

    let arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 1 })
        .collect()
        .await;

    assert_eq!(
        arrivals,
        vec![
            Ok("Begrüßung".to_owned()),
            Ok("Anna am Tag 1".to_owned()),
            Ok("Ben am Tag 1".to_owned()),
            Ok("Clara am Tag 1".to_owned()),
            Ok("Abschied".to_owned())
        ]
    );
}

#[tokio::test]
async fn unread_stream_runs_no_stream_behavior() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_stream_behavior(concierge(&journal))
        .register_stream(Handler::instance(FrontOffice {
            guests: vec!["Anna".to_owned()],
            journal: journal.clone(),
        }))
        .build();

    let arrivals = mediator.create_stream(ListArrivals { day: 4 });
    drop(arrivals);

    assert_eq!(journal.entries(), Vec::<String>::new());
}

#[tokio::test]
async fn stream_behavior_not_reading_next_stream_keeps_stream_handler_from_running() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .stream_behavior::<ListArrivals>(ClosedDesk)
        .register_stream(Handler::instance(FrontOffice {
            guests: vec!["Anna".to_owned()],
            journal: journal.clone(),
        }))
        .build();

    let arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 6 })
        .collect()
        .await;

    assert_eq!(
        (arrivals, journal.entries()),
        (Vec::new(), Vec::<String>::new())
    );
}

#[tokio::test]
async fn same_stream_behavior_type_registered_twice_runs_once_per_stream() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_stream_behavior(concierge(&journal))
        .open_stream_behavior(concierge(&journal))
        .register_stream(Handler::instance(FrontOffice {
            guests: Vec::new(),
            journal: journal.clone(),
        }))
        .build();

    let _arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 7 })
        .collect()
        .await;

    assert_eq!(
        journal.entries(),
        vec![
            "Concierge vorher".to_owned(),
            "Anreisen für Tag 7 nachgeschlagen".to_owned(),
            "Concierge nachher".to_owned()
        ]
    );
}

#[tokio::test]
async fn behavior_for_all_request_types_does_not_wrap_stream() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_behavior(DoorLog {
            journal: journal.clone(),
        })
        .register_stream(Handler::instance(FrontOffice {
            guests: Vec::new(),
            journal: journal.clone(),
        }))
        .build();

    let _arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 8 })
        .collect()
        .await;

    assert_eq!(
        journal.entries(),
        vec!["Anreisen für Tag 8 nachgeschlagen".to_owned()]
    );
}

#[tokio::test]
async fn stream_behavior_for_all_stream_request_types_does_not_wrap_send() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_stream_behavior(concierge(&journal))
        .register(Handler::instance(CheckInDesk {
            journal: journal.clone(),
        }))
        .build();

    let check_in = mediator.send(CheckIn { room: 5 }).await;

    assert_eq!(
        (check_in, journal.entries()),
        (Ok(()), vec!["Check-in Zimmer 5".to_owned()])
    );
}

#[tokio::test]
async fn behavior_for_one_request_type_does_not_wrap_stream() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .behavior::<CheckIn>(DoorLog {
            journal: journal.clone(),
        })
        .register_stream(Handler::instance(FrontOffice {
            guests: Vec::new(),
            journal: journal.clone(),
        }))
        .build();

    let _arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 9 })
        .collect()
        .await;

    assert_eq!(
        journal.entries(),
        vec!["Anreisen für Tag 9 nachgeschlagen".to_owned()]
    );
}

#[tokio::test]
async fn stream_behavior_for_one_stream_request_type_does_not_wrap_send() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .stream_behavior::<ListArrivals>(concierge(&journal))
        .register(Handler::instance(CheckInDesk {
            journal: journal.clone(),
        }))
        .build();

    let check_in = mediator.send(CheckIn { room: 6 }).await;

    assert_eq!(
        (check_in, journal.entries()),
        (Ok(()), vec!["Check-in Zimmer 6".to_owned()])
    );
}
