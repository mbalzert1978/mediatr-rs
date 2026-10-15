//! Tests für den Abschnitt „Pipeline — Fehler-Handler und Fehler-Aktionen“ in `TESTLIST.md`.

use std::cell::RefCell;
use std::rc::Rc;

use mediatr_rs::{Handler, Mediator, Sender};

mod error_handlers_fixtures;

use error_handlers_fixtures::{
    BookRoom, BookingAlert, BookingError, BrokenAlarm, ClosedWaitingList, FullHotel, GuestNotice,
    OpenHotel, OverbookingCheck, PartnerHotel, PaymentGate, RenovationCheck, WaitingList,
};

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

#[tokio::test]
async fn two_error_actions_both_run_on_handler_error_and_send_returns_handler_error() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_request_error_action(BookingAlert {
            journal: Rc::clone(&journal),
        })
        .open_request_error_action(GuestNotice {
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
                "Alarm: Buchung für Zimmer 12 gescheitert: FullyBooked".to_owned(),
                "Gast informiert: Zimmer 12 nicht verfügbar".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn first_error_handler_replacement_is_returned_and_second_error_handler_does_not_run() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_request_error_handler(WaitingList {
            journal: Rc::clone(&journal),
        })
        .open_request_error_handler(PartnerHotel {
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
            Ok(9000),
            vec![
                "Zimmer 12 ausgebucht".to_owned(),
                "Zimmer 12 auf der Warteliste".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn error_handler_without_replacement_lets_send_return_original_error() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_request_error_handler(OverbookingCheck {
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
                "Überbuchung für Zimmer 12 abgelehnt".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn failing_error_handler_makes_send_return_its_error_instead_of_original() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_request_error_handler(ClosedWaitingList {
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
            Err(BookingError::WaitingListClosed),
            vec![
                "Zimmer 12 ausgebucht".to_owned(),
                "Warteliste für Zimmer 12 geschlossen".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn error_handler_without_replacement_runs_once_per_send_on_handler_error() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_request_error_handler(OverbookingCheck {
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
            "Überbuchung für Zimmer 12 abgelehnt".to_owned(),
            "Überbuchung für Zimmer 14 abgelehnt".to_owned()
        ]
    );
}

#[tokio::test]
async fn error_handler_replacement_is_returned_and_error_action_does_not_run() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_request_error_action(BookingAlert {
            journal: Rc::clone(&journal),
        })
        .open_request_error_handler(WaitingList {
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
            Ok(9000),
            vec![
                "Zimmer 12 ausgebucht".to_owned(),
                "Zimmer 12 auf der Warteliste".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn error_handler_without_replacement_and_error_action_both_run_and_send_returns_handler_error()
 {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_request_error_action(BookingAlert {
            journal: Rc::clone(&journal),
        })
        .open_request_error_handler(OverbookingCheck {
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
                "Überbuchung für Zimmer 12 abgelehnt".to_owned(),
                "Alarm: Buchung für Zimmer 12 gescheitert: FullyBooked".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn failing_behavior_before_handler_runs_error_handler_and_error_action_and_send_returns_its_error()
 {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_behavior(PaymentGate {
            journal: Rc::clone(&journal),
        })
        .open_request_error_action(BookingAlert {
            journal: Rc::clone(&journal),
        })
        .open_request_error_handler(OverbookingCheck {
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
            Err(BookingError::PaymentDeclined),
            vec![
                "Zahlung für Zimmer 12 abgelehnt".to_owned(),
                "Überbuchung für Zimmer 12 abgelehnt".to_owned(),
                "Alarm: Buchung für Zimmer 12 gescheitert: PaymentDeclined".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn failing_error_action_makes_send_return_its_error_instead_of_handler_error() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_request_error_action(BrokenAlarm {
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
            Err(BookingError::AlarmFailed),
            vec![
                "Zimmer 12 ausgebucht".to_owned(),
                "Alarm für Zimmer 12 ausgefallen".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn successful_handler_skips_error_action_and_send_returns_its_response() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_request_error_action(BookingAlert {
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(OpenHotel {
            journal: Rc::clone(&journal),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.borrow().clone()),
        (Ok(4711), vec!["Zimmer 12 gebucht".to_owned()])
    );
}

#[tokio::test]
async fn failing_pre_processor_runs_error_action_and_send_returns_its_error() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .open_request_pre_processor(RenovationCheck {
            journal: Rc::clone(&journal),
        })
        .open_request_error_action(BookingAlert {
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(OpenHotel {
            journal: Rc::clone(&journal),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.borrow().clone()),
        (
            Err(BookingError::UnderRenovation),
            vec![
                "Zimmer 12 wird renoviert".to_owned(),
                "Alarm: Buchung für Zimmer 12 gescheitert: UnderRenovation".to_owned()
            ]
        )
    );
}
