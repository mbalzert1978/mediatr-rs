//! Tests für den Abschnitt „Stream-Request — Stream-Behaviors“ in `TESTLIST.md`.

use std::cell::RefCell;
use std::rc::Rc;

use futures_util::StreamExt;
use mediatr_rs::{Handler, Mediator, StreamSender};

mod stream_behaviors_fixtures;

use stream_behaviors_fixtures::{
    Concierge, FrontOffice, ListArrivals, NightPorter, concierge_and_night_porter_around,
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
