//! Tests für den Abschnitt „Pipeline — Behaviors“ in `TESTLIST.md`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use mediatr_rs::{Concurrent, Handler, Mediator, MediatorBuilder, Publisher, Sender};

mod behaviors_fixtures;

use behaviors_fixtures::{
    AuditTrail, BookReceipt, LoadingDock, LookupStock, ReceiptBook, ReorderCheck, RetryOnce,
    ShelfLabeling, ShelfLock, SlowStockLedger, StockCounted, StockLedger, audit_and_stopwatch,
    audited_and_timed, audited_locked_and_timed, checkpoint, labeled_and_reordered_together,
    passed_checkpoints,
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

#[tokio::test]
async fn unit_response_behavior_for_one_request_type_then_one_for_all_wrap_handler_first_registered_outermost()
 {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .behavior_for::<BookReceipt>(LoadingDock {
            journal: Rc::clone(&journal),
        })
        .behavior(AuditTrail {
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
            [
                "Rampe belegt",
                "Prüfung beginnt",
                "Eingang gebucht für 9",
                "Prüfung endet",
                "Rampe frei"
            ]
            .map(str::to_owned)
            .to_vec()
        )
    );
}

#[tokio::test]
async fn twelve_behaviors_wrap_handler_first_registered_outermost() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .behavior(checkpoint::<0>(&journal))
        .behavior(checkpoint::<1>(&journal))
        .behavior(checkpoint::<2>(&journal))
        .behavior(checkpoint::<3>(&journal))
        .behavior(checkpoint::<4>(&journal))
        .behavior(checkpoint::<5>(&journal))
        .behavior(checkpoint::<6>(&journal))
        .behavior(checkpoint::<7>(&journal))
        .behavior(checkpoint::<8>(&journal))
        .behavior(checkpoint::<9>(&journal))
        .behavior(checkpoint::<10>(&journal))
        .behavior(checkpoint::<11>(&journal))
        .register(Handler::instance(StockLedger {
            journal: Rc::clone(&journal),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.borrow().clone()),
        (Ok(12), passed_checkpoints(12, "Bestand gelesen für 4"))
    );
}

#[tokio::test]
async fn behaviors_with_concurrent_strategy_wrap_send_and_publish_starts_handlers_together() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let (audit, stopwatch) = audit_and_stopwatch(&journal);
    let mediator = MediatorBuilder::<Concurrent>::default()
        .behavior(audit)
        .behavior(stopwatch)
        .register(Handler::instance(StockLedger {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(ShelfLabeling {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(ReorderCheck {
            journal: Rc::clone(&journal),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;
    let counted = mediator.publish(StockCounted { sku: 4 }).await;

    assert_eq!(
        (stock, counted, journal.borrow().clone()),
        (
            Ok(12),
            Ok(()),
            [
                audited_and_timed("Bestand gelesen für 4"),
                labeled_and_reordered_together(4),
            ]
            .concat()
        )
    );
}

#[tokio::test]
async fn transient_handler_wrapped_by_behavior_is_built_once_per_send_and_wrapped_once() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let builds = Rc::new(Cell::new(0_u32));
    let ledger_journal = Rc::clone(&journal);
    let ledger_builds = Rc::clone(&builds);
    let mediator = Mediator::builder()
        .behavior(AuditTrail {
            journal: Rc::clone(&journal),
        })
        .register(Handler::transient(move |_sender| {
            ledger_builds.set(ledger_builds.get().saturating_add(1));
            StockLedger {
                journal: Rc::clone(&ledger_journal),
            }
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, builds.get(), journal.borrow().clone()),
        (
            Ok(12),
            1,
            ["Prüfung beginnt", "Bestand gelesen für 4", "Prüfung endet"]
                .map(str::to_owned)
                .to_vec()
        )
    );
}
