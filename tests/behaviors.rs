//! Tests für den Abschnitt „Pipeline — Behaviors“ in `TESTLIST.md`.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use mediatr_rs::{
    Handler, Mediator, MediatorBuilder, Publisher, SendError, Sender, WhenAllPublisher,
};

mod behaviors_fixtures;
mod common;

use behaviors_fixtures::{
    AuditTrail, BookReceipt, Deadline, LoadingDock, LookupStock, ReceiptBook, RemoteReceiving,
    RemoteWarehouse, ReorderCheck, RetryOnce, RetryWithCopy, ShelfLabeling, ShelfLock,
    SlowStockLedger, StockCache, StockCounted, StockError, StockLedger, audit_and_stopwatch,
    audited_and_timed, audited_locked_and_timed, checkpoint, labeled_and_reordered_together,
    passed_checkpoints,
};
use common::Journal;

#[tokio::test]
async fn behavior_retries_cloned_next_after_handler_error_and_returns_second_response() {
    let calls = Arc::new(AtomicU32::new(0));
    let mediator = Mediator::builder()
        .open_behavior(RetryOnce)
        .register(Handler::instance(SlowStockLedger {
            calls: Arc::clone(&calls),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!((stock, calls.load(Ordering::SeqCst)), (Ok(12), 2));
}

#[tokio::test]
async fn two_behaviors_wrap_handler_first_registered_outermost() {
    let journal = Journal::default();
    let (audit, stopwatch) = audit_and_stopwatch(&journal);
    let mediator = Mediator::builder()
        .open_behavior(audit)
        .open_behavior(stopwatch)
        .register(Handler::instance(StockLedger {
            journal: journal.clone(),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.entries()),
        (Ok(12), audited_and_timed("Bestand gelesen für 4"))
    );
}

#[tokio::test]
async fn two_behaviors_wrap_unit_response_handler_first_registered_outermost() {
    let journal = Journal::default();
    let (audit, stopwatch) = audit_and_stopwatch(&journal);
    let mediator = Mediator::builder()
        .open_behavior(audit)
        .open_behavior(stopwatch)
        .register(Handler::instance(ReceiptBook {
            journal: journal.clone(),
        }))
        .build();

    let booked = mediator.send(BookReceipt { sku: 4 }).await;

    assert_eq!(
        (booked, journal.entries()),
        (Ok(()), audited_and_timed("Eingang gebucht für 4"))
    );
}

#[tokio::test]
async fn two_behaviors_wrap_handlers_of_every_request_type_first_registered_outermost() {
    let journal = Journal::default();
    let (audit, stopwatch) = audit_and_stopwatch(&journal);
    let mediator = Mediator::builder()
        .open_behavior(audit)
        .open_behavior(stopwatch)
        .register(Handler::instance(StockLedger {
            journal: journal.clone(),
        }))
        .register(Handler::instance(ReceiptBook {
            journal: journal.clone(),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;
    let booked = mediator.send(BookReceipt { sku: 9 }).await;

    assert_eq!(
        (stock, booked, journal.entries()),
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
async fn behavior_between_two_open_behaviors_wraps_only_its_requests_in_registration_order() {
    let journal = Journal::default();
    let (audit, stopwatch) = audit_and_stopwatch(&journal);
    let mediator = Mediator::builder()
        .open_behavior(audit)
        .behavior::<LookupStock>(ShelfLock {
            journal: journal.clone(),
        })
        .open_behavior(stopwatch)
        .register(Handler::instance(StockLedger {
            journal: journal.clone(),
        }))
        .register(Handler::instance(ReceiptBook {
            journal: journal.clone(),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;
    let booked = mediator.send(BookReceipt { sku: 9 }).await;

    assert_eq!(
        (stock, booked, journal.entries()),
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
    let journal = Journal::default();
    let (audit, stopwatch) = audit_and_stopwatch(&journal);
    let (second_audit, second_stopwatch) = audit_and_stopwatch(&journal);
    let (third_audit, third_stopwatch) = audit_and_stopwatch(&journal);
    let mediator = Mediator::builder()
        .open_behavior(audit)
        .open_behavior(stopwatch)
        .open_behavior(second_audit)
        .open_behavior(second_stopwatch)
        .open_behavior(third_audit)
        .open_behavior(third_stopwatch)
        .register(Handler::instance(StockLedger {
            journal: journal.clone(),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.entries()),
        (Ok(12), audited_and_timed("Bestand gelesen für 4"))
    );
}

#[tokio::test]
async fn same_behavior_registered_twice_for_one_request_type_runs_once() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .behavior::<LookupStock>(ShelfLock {
            journal: journal.clone(),
        })
        .behavior::<LookupStock>(ShelfLock {
            journal: journal.clone(),
        })
        .register(Handler::instance(StockLedger {
            journal: journal.clone(),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.entries()),
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
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_behavior(AuditTrail {
            journal: journal.clone(),
        })
        .behavior::<LookupStock>(AuditTrail {
            journal: journal.clone(),
        })
        .register(Handler::instance(StockLedger {
            journal: journal.clone(),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.entries()),
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
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .behavior::<LookupStock>(AuditTrail {
            journal: journal.clone(),
        })
        .open_behavior(AuditTrail {
            journal: journal.clone(),
        })
        .register(Handler::instance(StockLedger {
            journal: journal.clone(),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.entries()),
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
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_behavior(AuditTrail {
            journal: journal.clone(),
        })
        .behavior::<LookupStock>(AuditTrail {
            journal: journal.clone(),
        })
        .register(Handler::instance(ReceiptBook {
            journal: journal.clone(),
        }))
        .build();

    let booked = mediator.send(BookReceipt { sku: 9 }).await;

    assert_eq!(
        (booked, journal.entries()),
        (
            Ok(()),
            ["Prüfung beginnt", "Eingang gebucht für 9", "Prüfung endet"]
                .map(str::to_owned)
                .to_vec()
        )
    );
}

#[tokio::test]
async fn unit_response_behavior_then_open_behavior_wrap_handler_first_registered_outermost() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .behavior::<BookReceipt>(LoadingDock {
            journal: journal.clone(),
        })
        .open_behavior(AuditTrail {
            journal: journal.clone(),
        })
        .register(Handler::instance(ReceiptBook {
            journal: journal.clone(),
        }))
        .build();

    let booked = mediator.send(BookReceipt { sku: 9 }).await;

    assert_eq!(
        (booked, journal.entries()),
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
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_behavior(checkpoint::<0>(&journal))
        .open_behavior(checkpoint::<1>(&journal))
        .open_behavior(checkpoint::<2>(&journal))
        .open_behavior(checkpoint::<3>(&journal))
        .open_behavior(checkpoint::<4>(&journal))
        .open_behavior(checkpoint::<5>(&journal))
        .open_behavior(checkpoint::<6>(&journal))
        .open_behavior(checkpoint::<7>(&journal))
        .open_behavior(checkpoint::<8>(&journal))
        .open_behavior(checkpoint::<9>(&journal))
        .open_behavior(checkpoint::<10>(&journal))
        .open_behavior(checkpoint::<11>(&journal))
        .register(Handler::instance(StockLedger {
            journal: journal.clone(),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.entries()),
        (Ok(12), passed_checkpoints(12, "Bestand gelesen für 4"))
    );
}

#[tokio::test]
async fn behaviors_with_when_all_publisher_wrap_send_and_publish_starts_handlers_together() {
    let journal = Journal::default();
    let (audit, stopwatch) = audit_and_stopwatch(&journal);
    let mediator = MediatorBuilder::<WhenAllPublisher>::default()
        .open_behavior(audit)
        .open_behavior(stopwatch)
        .register(Handler::instance(StockLedger {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(ShelfLabeling {
            journal: journal.clone(),
        }))
        .subscribe(Handler::instance(ReorderCheck {
            journal: journal.clone(),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;
    let counted = mediator.publish(StockCounted { sku: 4 }).await;

    assert_eq!(
        (stock, counted, journal.entries()),
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
    let journal = Journal::default();
    let builds = Arc::new(AtomicU32::new(0));
    let ledger_journal = journal.clone();
    let ledger_builds = Arc::clone(&builds);
    let mediator = Mediator::builder()
        .open_behavior(AuditTrail {
            journal: journal.clone(),
        })
        .register(Handler::transient(move |_sender| {
            ledger_builds.fetch_add(1, Ordering::SeqCst);
            StockLedger {
                journal: ledger_journal.clone(),
            }
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, builds.load(Ordering::SeqCst), journal.entries()),
        (
            Ok(12),
            1,
            ["Prüfung beginnt", "Bestand gelesen für 4", "Prüfung endet"]
                .map(str::to_owned)
                .to_vec()
        )
    );
}

#[tokio::test]
async fn behavior_answering_without_next_returns_its_response_and_handler_does_not_run() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .behavior::<LookupStock>(StockCache { cached: 7 })
        .register(Handler::instance(StockLedger {
            journal: journal.clone(),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!((stock, journal.entries().len()), (Ok(7), 0));
}

#[tokio::test(start_paused = true)]
async fn behavior_deadline_shorter_than_handler_returns_err_and_handler_does_not_finish() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .behavior::<LookupStock>(Deadline {
            limit: Duration::from_secs(5),
        })
        .register(Handler::instance(RemoteWarehouse {
            journal: journal.clone(),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, journal.entries()),
        (
            Err(StockError::Timeout),
            vec!["Außenlager sucht 4".to_owned()]
        )
    );
}

#[tokio::test(start_paused = true)]
async fn behavior_deadline_shorter_than_unit_response_handler_returns_err_and_handler_does_not_finish()
 {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .behavior::<BookReceipt>(Deadline {
            limit: Duration::from_secs(5),
        })
        .register(Handler::instance(RemoteReceiving {
            journal: journal.clone(),
        }))
        .build();

    let booked = mediator.send(BookReceipt { sku: 9 }).await;

    assert_eq!(
        (booked, journal.entries()),
        (
            Err(StockError::Timeout),
            vec!["Außenlager bucht 9".to_owned()]
        )
    );
}

#[tokio::test]
async fn behavior_requiring_clone_with_non_clone_request_without_handler_returns_no_handler() {
    let attempts = Arc::new(AtomicU32::new(0));
    let mediator = Mediator::builder()
        .open_behavior(RetryWithCopy {
            attempts: Arc::clone(&attempts),
        })
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!(
        (stock, attempts.load(Ordering::SeqCst)),
        (Err(StockError::Undeliverable(SendError::NoHandler)), 0)
    );
}
