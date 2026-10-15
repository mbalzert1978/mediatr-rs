//! Tests für den Abschnitt „Pipeline — Fehler-Handler und Fehler-Aktionen“ in `TESTLIST.md`.

use std::cell::RefCell;
use std::rc::Rc;

use mediatr_rs::{Handler, Mediator, Sender};

mod error_handlers_fixtures;

use error_handlers_fixtures::{BookRoom, BookingAlert, BookingError, FullHotel};

#[tokio::test]
async fn error_action_runs_on_handler_error_and_send_returns_handler_error() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_request_error_action(BookingAlert {
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(FullHotel {
            journal: Rc::clone(&journal),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.borrow().clone()),
        (
            Err(BookingError::FullyBooked),
            vec![
                "Zimmer 12 ausgebucht".to_owned(),
                "Alarm: Buchung für Zimmer 12 gescheitert: FullyBooked".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn error_action_runs_once_per_send_on_handler_error() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_request_error_action(BookingAlert {
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(FullHotel {
            journal: Rc::new(RefCell::new(Vec::new())),
        }))
        .build();

    let _first = mediator.send(BookRoom { room: 12 }).await;
    let _second = mediator.send(BookRoom { room: 14 }).await;

    assert_eq!(
        journal.borrow().clone(),
        vec![
            "Alarm: Buchung für Zimmer 12 gescheitert: FullyBooked".to_owned(),
            "Alarm: Buchung für Zimmer 14 gescheitert: FullyBooked".to_owned()
        ]
    );
}
