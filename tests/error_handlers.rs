//! Tests für den Abschnitt „Pipeline — Fehler-Handler und Fehler-Aktionen“ in `TESTLIST.md`.

use mediatr_rs::{Handler, Mediator, Sender};

mod common;
mod error_handlers_fixtures;

use common::Journal;
use error_handlers_fixtures::{
    BookRoom, BookingAlert, BookingError, BrokenAlarm, CancelBooking, ClosedWaitingList,
    ConfirmationMail, FrontDesk, FullHotel, GuestNotice, NoRefunds, OpenHotel, OverbookingCheck,
    PartnerHotel, PaymentGate, RenovationCheck, WaitingList,
};

#[tokio::test]
async fn error_action_runs_on_handler_error_and_send_returns_handler_error() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_action(BookingAlert {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.entries()),
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
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_action(BookingAlert {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: Journal::default(),
        }))
        .build();

    let _first = mediator.send(BookRoom { room: 12 }).await;
    let _second = mediator.send(BookRoom { room: 14 }).await;

    assert_eq!(
        journal.entries(),
        vec![
            "Alarm: Buchung für Zimmer 12 gescheitert: FullyBooked".to_owned(),
            "Alarm: Buchung für Zimmer 14 gescheitert: FullyBooked".to_owned()
        ]
    );
}

#[tokio::test]
async fn two_error_actions_both_run_on_handler_error_and_send_returns_handler_error() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_action(BookingAlert {
            journal: journal.clone(),
        })
        .open_request_error_action(GuestNotice {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.entries()),
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
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_handler(WaitingList {
            journal: journal.clone(),
        })
        .open_request_error_handler(PartnerHotel {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.entries()),
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
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_handler(OverbookingCheck {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.entries()),
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
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_handler(ClosedWaitingList {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.entries()),
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
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_handler(OverbookingCheck {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: Journal::default(),
        }))
        .build();

    let _first = mediator.send(BookRoom { room: 12 }).await;
    let _second = mediator.send(BookRoom { room: 14 }).await;

    assert_eq!(
        journal.entries(),
        vec![
            "Überbuchung für Zimmer 12 abgelehnt".to_owned(),
            "Überbuchung für Zimmer 14 abgelehnt".to_owned()
        ]
    );
}

#[tokio::test]
async fn error_handler_replacement_is_returned_and_error_action_does_not_run() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_action(BookingAlert {
            journal: journal.clone(),
        })
        .open_request_error_handler(WaitingList {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.entries()),
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
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_action(BookingAlert {
            journal: journal.clone(),
        })
        .open_request_error_handler(OverbookingCheck {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.entries()),
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
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_behavior(PaymentGate {
            journal: journal.clone(),
        })
        .open_request_error_action(BookingAlert {
            journal: journal.clone(),
        })
        .open_request_error_handler(OverbookingCheck {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.entries()),
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
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_action(BrokenAlarm {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.entries()),
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
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_action(BookingAlert {
            journal: journal.clone(),
        })
        .register(Handler::instance(OpenHotel {
            journal: journal.clone(),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.entries()),
        (Ok(4711), vec!["Zimmer 12 gebucht".to_owned()])
    );
}

#[tokio::test]
async fn failing_pre_processor_runs_error_action_and_send_returns_its_error() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_pre_processor(RenovationCheck {
            journal: journal.clone(),
        })
        .open_request_error_action(BookingAlert {
            journal: journal.clone(),
        })
        .register(Handler::instance(OpenHotel {
            journal: journal.clone(),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.entries()),
        (
            Err(BookingError::UnderRenovation),
            vec![
                "Zimmer 12 wird renoviert".to_owned(),
                "Alarm: Buchung für Zimmer 12 gescheitert: UnderRenovation".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn failing_post_processor_runs_error_action_and_send_returns_its_error() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_post_processor(ConfirmationMail {
            journal: journal.clone(),
        })
        .open_request_error_action(BookingAlert {
            journal: journal.clone(),
        })
        .register(Handler::instance(OpenHotel {
            journal: journal.clone(),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.entries()),
        (
            Err(BookingError::ConfirmationFailed),
            vec![
                "Zimmer 12 gebucht".to_owned(),
                "Bestätigung für Buchung 4711 nicht versendet".to_owned(),
                "Alarm: Buchung für Zimmer 12 gescheitert: ConfirmationFailed".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn second_error_handler_gets_error_passed_on_by_first_instead_of_original() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_handler(ClosedWaitingList {
            journal: journal.clone(),
        })
        .open_request_error_handler(FrontDesk {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .build();

    let _booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        journal.entries(),
        vec![
            "Zimmer 12 ausgebucht".to_owned(),
            "Warteliste für Zimmer 12 geschlossen".to_owned(),
            "Rezeption meldet für Zimmer 12: WaitingListClosed".to_owned()
        ]
    );
}

#[tokio::test]
async fn successful_handler_skips_error_handler_and_send_returns_its_response() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_handler(WaitingList {
            journal: journal.clone(),
        })
        .register(Handler::instance(OpenHotel {
            journal: journal.clone(),
        }))
        .build();

    let booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        (booking, journal.entries()),
        (Ok(4711), vec!["Zimmer 12 gebucht".to_owned()])
    );
}

#[tokio::test]
async fn same_error_action_type_registered_twice_runs_once_per_send() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_action(BookingAlert {
            journal: journal.clone(),
        })
        .open_request_error_action(BookingAlert {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .build();

    let _booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        journal.entries(),
        vec![
            "Zimmer 12 ausgebucht".to_owned(),
            "Alarm: Buchung für Zimmer 12 gescheitert: FullyBooked".to_owned()
        ]
    );
}

#[tokio::test]
async fn same_error_handler_type_registered_twice_runs_once_per_send() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_error_handler(OverbookingCheck {
            journal: journal.clone(),
        })
        .open_request_error_handler(OverbookingCheck {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .build();

    let _booking = mediator.send(BookRoom { room: 12 }).await;

    assert_eq!(
        journal.entries(),
        vec![
            "Zimmer 12 ausgebucht".to_owned(),
            "Überbuchung für Zimmer 12 abgelehnt".to_owned()
        ]
    );
}

#[tokio::test]
async fn error_action_for_one_request_type_runs_only_for_that_type() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .request_error_action::<BookRoom>(BookingAlert {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .register(Handler::instance(NoRefunds {
            journal: journal.clone(),
        }))
        .build();

    let _booking = mediator.send(BookRoom { room: 12 }).await;
    let cancellation = mediator.send(CancelBooking { booking: 4711 }).await;

    assert_eq!(
        (cancellation, journal.entries()),
        (
            Err(BookingError::NotRefundable),
            vec![
                "Zimmer 12 ausgebucht".to_owned(),
                "Alarm: Buchung für Zimmer 12 gescheitert: FullyBooked".to_owned(),
                "Storno für Buchung 4711 abgelehnt".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn error_handler_for_one_request_type_runs_only_for_that_type() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .request_error_handler::<BookRoom>(OverbookingCheck {
            journal: journal.clone(),
        })
        .register(Handler::instance(FullHotel {
            journal: journal.clone(),
        }))
        .register(Handler::instance(NoRefunds {
            journal: journal.clone(),
        }))
        .build();

    let _booking = mediator.send(BookRoom { room: 12 }).await;
    let cancellation = mediator.send(CancelBooking { booking: 4711 }).await;

    assert_eq!(
        (cancellation, journal.entries()),
        (
            Err(BookingError::NotRefundable),
            vec![
                "Zimmer 12 ausgebucht".to_owned(),
                "Überbuchung für Zimmer 12 abgelehnt".to_owned(),
                "Storno für Buchung 4711 abgelehnt".to_owned()
            ]
        )
    );
}
