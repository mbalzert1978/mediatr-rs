//! Tests für den Abschnitt „Stream-Request — Stream-Behaviors“ in `TESTLIST.md`.

use std::cell::RefCell;
use std::rc::Rc;

use futures_util::StreamExt;
use mediatr_rs::{Handler, Mediator, StreamSender};

mod stream_behaviors_fixtures;

use stream_behaviors_fixtures::{
    Concierge, DepartureBoard, FrontOffice, Greeter, ListArrivals, ListDepartures, NightPorter,
    bellboy, concierge, concierge_and_night_porter_around,
    concierge_bellboy_and_night_porter_around, night_porter,
};

#[tokio::test]
async fn two_open_stream_behaviors_wrap_handler_first_registered_outside_items_unchanged() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_stream_behavior(Concierge {
            journal: Rc::clone(&journal),
        })
        .open_stream_behavior(NightPorter {
            journal: Rc::clone(&journal),
        })
        .register_stream(Handler::instance(FrontOffice {
            guests: vec!["Anna".to_owned(), "Ben".to_owned()],
            journal: Rc::clone(&journal),
        }))
        .build();

    let arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 5 })
        .collect()
        .await;

    assert_eq!(
        (arrivals, journal.borrow().clone()),
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
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .stream_behavior::<ListArrivals>(Concierge {
            journal: Rc::clone(&journal),
        })
        .stream_behavior::<ListArrivals>(NightPorter {
            journal: Rc::clone(&journal),
        })
        .register_stream(Handler::instance(FrontOffice {
            guests: vec!["Clara".to_owned()],
            journal: Rc::clone(&journal),
        }))
        .build();

    let arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 2 })
        .collect()
        .await;

    assert_eq!(
        (arrivals, journal.borrow().clone()),
        (
            vec![Ok("Clara am Tag 2".to_owned())],
            concierge_and_night_porter_around("Anreisen für Tag 2 nachgeschlagen")
        )
    );
}

#[tokio::test]
async fn stream_behavior_for_one_type_joins_general_ones_in_registration_order_other_type_gets_only_general()
 {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_stream_behavior(concierge(&journal))
        .stream_behavior::<ListArrivals>(bellboy(&journal))
        .open_stream_behavior(night_porter(&journal))
        .register_stream(Handler::instance(FrontOffice {
            guests: Vec::new(),
            journal: Rc::clone(&journal),
        }))
        .register_stream(Handler::instance(DepartureBoard {
            journal: Rc::clone(&journal),
        }))
        .build();

    let _arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 3 })
        .collect()
        .await;
    let arrivals_journal = journal.take();
    let _departures: Vec<_> = mediator
        .create_stream(ListDepartures { day: 3 })
        .collect()
        .await;

    assert_eq!(
        (arrivals_journal, journal.take()),
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
            journal: Rc::new(RefCell::new(Vec::new())),
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
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_stream_behavior(concierge(&journal))
        .register_stream(Handler::instance(FrontOffice {
            guests: vec!["Anna".to_owned()],
            journal: Rc::clone(&journal),
        }))
        .build();

    let arrivals = mediator.create_stream(ListArrivals { day: 4 });
    drop(arrivals);

    assert_eq!(journal.borrow().clone(), Vec::<String>::new());
}
