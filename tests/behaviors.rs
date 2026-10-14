//! Tests für den Abschnitt „Pipeline — Behaviors“ in `TESTLIST.md`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use mediatr_rs::{Handler, Mediator, Sender};

mod behaviors_fixtures;

use behaviors_fixtures::{
    AuditTrail, BookReceipt, LookupStock, ReceiptBook, RetryOnce, ShelfLock, SlowStockLedger,
    StockLedger, audit_and_stopwatch, audited_and_timed, audited_locked_and_timed,
};

#[tokio::test]
async fn behavior_retries_cloned_next_after_handler_error_and_returns_second_response() {
    let calls = Rc::new(Cell::new(0));
    let mediator = Mediator::builder()
        .behavior(RetryOnce)
        .register(Handler::instance(SlowStockLedger {
            calls: Rc::clone(&calls),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!((stock, calls.get()), (Ok(12), 2));
}

#[tokio::test]
async fn two_behaviors_wrap_handler_first_registered_outermost() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let (audit, stopwatch) = audit_and_stopwatch(&journal);
    let mediator = Mediator::builder()
        .behavior(audit)
        .behavior(stopwatch)
        .register(Handler::instance(StockLedger {
            journal: Rc::clone(&journal),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.borrow().clone()),
        (Ok(12), audited_and_timed("Bestand gelesen für 4"))
    );
}

#[tokio::test]
async fn two_behaviors_wrap_unit_response_handler_first_registered_outermost() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let (audit, stopwatch) = audit_and_stopwatch(&journal);
    let mediator = Mediator::builder()
        .behavior(audit)
        .behavior(stopwatch)
        .register(Handler::instance(ReceiptBook {
            journal: Rc::clone(&journal),
        }))
        .build();

    let booked = mediator.send(BookReceipt { sku: 4 }).await;

    assert_eq!(
        (booked, journal.borrow().clone()),
        (Ok(()), audited_and_timed("Eingang gebucht für 4"))
    );
}

#[tokio::test]
async fn two_behaviors_wrap_handlers_of_every_request_type_first_registered_outermost() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let (audit, stopwatch) = audit_and_stopwatch(&journal);
    let mediator = Mediator::builder()
        .behavior(audit)
        .behavior(stopwatch)
        .register(Handler::instance(StockLedger {
            journal: Rc::clone(&journal),
        }))
        .register(Handler::instance(ReceiptBook {
            journal: Rc::clone(&journal),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;
    let booked = mediator.send(BookReceipt { sku: 9 }).await;

    assert_eq!(
        (stock, booked, journal.borrow().clone()),
        (
            Ok(12),
            Ok(()),
            [
                audited_and_timed("Bestand gelesen für 4"),
                audited_and_timed("Eingang gebucht für 9")
            ]
            .concat()
        )
    );
}

#[tokio::test]
async fn behavior_for_one_request_type_between_two_for_all_wraps_only_its_requests_in_registration_order()
 {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let (audit, stopwatch) = audit_and_stopwatch(&journal);
    let mediator = Mediator::builder()
        .behavior(audit)
        .behavior_for::<LookupStock>(ShelfLock {
            journal: Rc::clone(&journal),
        })
        .behavior(stopwatch)
        .register(Handler::instance(StockLedger {
            journal: Rc::clone(&journal),
        }))
        .register(Handler::instance(ReceiptBook {
            journal: Rc::clone(&journal),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;
    let booked = mediator.send(BookReceipt { sku: 9 }).await;

    assert_eq!(
        (stock, booked, journal.borrow().clone()),
        (
            Ok(12),
            Ok(()),
            [
                audited_locked_and_timed("Bestand gelesen für 4"),
                audited_and_timed("Eingang gebucht für 9")
            ]
            .concat()
        )
    );
}

#[tokio::test]
async fn two_behaviors_registered_three_times_alternately_run_once_in_first_registration_order() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let (audit, stopwatch) = audit_and_stopwatch(&journal);
    let (second_audit, second_stopwatch) = audit_and_stopwatch(&journal);
    let (third_audit, third_stopwatch) = audit_and_stopwatch(&journal);
    let mediator = Mediator::builder()
        .behavior(audit)
        .behavior(stopwatch)
        .behavior(second_audit)
        .behavior(second_stopwatch)
        .behavior(third_audit)
        .behavior(third_stopwatch)
        .register(Handler::instance(StockLedger {
            journal: Rc::clone(&journal),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.borrow().clone()),
        (Ok(12), audited_and_timed("Bestand gelesen für 4"))
    );
}

#[tokio::test]
async fn same_behavior_registered_twice_for_one_request_type_runs_once() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .behavior_for::<LookupStock>(ShelfLock {
            journal: Rc::clone(&journal),
        })
        .behavior_for::<LookupStock>(ShelfLock {
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(StockLedger {
            journal: Rc::clone(&journal),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.borrow().clone()),
        (
            Ok(12),
            ["Regal gesperrt", "Bestand gelesen für 4", "Regal frei"]
                .map(str::to_owned)
                .to_vec()
        )
    );
}

#[tokio::test]
async fn same_behavior_registered_for_all_and_for_one_request_type_runs_twice_for_that_type() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .behavior(AuditTrail {
            journal: Rc::clone(&journal),
        })
        .behavior_for::<LookupStock>(AuditTrail {
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(StockLedger {
            journal: Rc::clone(&journal),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.borrow().clone()),
        (
            Ok(12),
            [
                "Prüfung beginnt",
                "Prüfung beginnt",
                "Bestand gelesen für 4",
                "Prüfung endet",
                "Prüfung endet"
            ]
            .map(str::to_owned)
            .to_vec()
        )
    );
}

#[tokio::test]
async fn same_behavior_registered_for_one_request_type_then_for_all_runs_twice_for_that_type() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .behavior_for::<LookupStock>(AuditTrail {
            journal: Rc::clone(&journal),
        })
        .behavior(AuditTrail {
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(StockLedger {
            journal: Rc::clone(&journal),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.borrow().clone()),
        (
            Ok(12),
            [
                "Prüfung beginnt",
                "Prüfung beginnt",
                "Bestand gelesen für 4",
                "Prüfung endet",
                "Prüfung endet"
            ]
            .map(str::to_owned)
            .to_vec()
        )
    );
}

#[tokio::test]
async fn same_behavior_registered_for_all_and_for_one_request_type_runs_once_for_another_type() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .behavior(AuditTrail {
            journal: Rc::clone(&journal),
        })
        .behavior_for::<LookupStock>(AuditTrail {
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(ReceiptBook {
            journal: Rc::clone(&journal),
        }))
        .build();

    let booked = mediator.send(BookReceipt { sku: 9 }).await;

    assert_eq!(
        (booked, journal.borrow().clone()),
        (
            Ok(()),
            ["Prüfung beginnt", "Eingang gebucht für 9", "Prüfung endet"]
                .map(str::to_owned)
                .to_vec()
        )
    );
}
