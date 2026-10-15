//! Tests für den Abschnitt „Pipeline — Vor- und Nachverarbeiter“ in `TESTLIST.md`.

use std::rc::Rc;

use mediatr_rs::{Handler, Mediator, Sender};

mod common;
mod processors_fixtures;

use common::Journal;
use processors_fixtures::{
    InquiryLog, PriceCeiling, PriceError, PriceList, QuoteArchive, QuotePrice, QuoteShipping,
    RetiredFilter, RetiredItems, ShippingTable, checked_and_exported, inbox_stamp, outbox_stamp,
    price_check, price_export, price_lock, price_through_all_stages,
    shipping_through_general_stages, timer,
};

#[tokio::test]
async fn pre_processor_runs_before_handler_with_request_and_send_returns_handler_response() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_pre_processor(InquiryLog {
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(PriceList {
            journal: Rc::clone(&journal),
        }))
        .build();

    let price = mediator.send(QuotePrice { sku: 7 }).await;

    assert_eq!(
        (price, journal.borrow().clone()),
        (
            Ok(1999),
            vec![
                "Anfrage für 7 eingegangen".to_owned(),
                "Preis berechnet für 7".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn post_processor_runs_after_handler_with_request_and_response_and_send_returns_handler_response()
 {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_post_processor(QuoteArchive {
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(PriceList {
            journal: Rc::clone(&journal),
        }))
        .build();

    let price = mediator.send(QuotePrice { sku: 7 }).await;

    assert_eq!(
        (price, journal.borrow().clone()),
        (
            Ok(1999),
            vec![
                "Preis berechnet für 7".to_owned(),
                "Preis 1999 für 7 archiviert".to_owned()
            ]
        )
    );
}

#[tokio::test]
async fn pre_and_post_processors_for_one_and_all_request_types_run_in_registration_order() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_pre_processor(price_check::<1>(&journal))
        .request_pre_processor::<QuotePrice>(price_check::<2>(&journal))
        .open_request_pre_processor(price_check::<3>(&journal))
        .request_pre_processor::<QuotePrice>(price_check::<4>(&journal))
        .request_post_processor::<QuotePrice>(price_export::<1>(&journal))
        .open_request_post_processor(price_export::<2>(&journal))
        .request_post_processor::<QuotePrice>(price_export::<3>(&journal))
        .open_request_post_processor(price_export::<4>(&journal))
        .register(Handler::instance(PriceList {
            journal: Rc::clone(&journal),
        }))
        .build();

    let price = mediator.send(QuotePrice { sku: 7 }).await;

    assert_eq!(
        (price, journal.borrow().clone()),
        (Ok(1999), checked_and_exported(4, "Preis berechnet für 7"))
    );
}

#[tokio::test]
async fn processors_run_outside_behaviors_and_other_request_only_through_general_ones() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_behavior(timer(&journal))
        .open_request_post_processor(outbox_stamp(&journal))
        .request_pre_processor::<QuotePrice>(price_check::<1>(&journal))
        .behavior::<QuotePrice>(price_lock(&journal))
        .open_request_pre_processor(inbox_stamp(&journal))
        .request_post_processor::<QuotePrice>(price_export::<1>(&journal))
        .register(Handler::instance(PriceList {
            journal: Rc::clone(&journal),
        }))
        .register(Handler::instance(ShippingTable {
            journal: Rc::clone(&journal),
        }))
        .build();

    let _price = mediator.send(QuotePrice { sku: 7 }).await;
    let price_journal = journal.take();
    let _shipping = mediator.send(QuoteShipping { sku: 7 }).await;

    assert_eq!(
        (price_journal, journal.take()),
        (
            price_through_all_stages(),
            shipping_through_general_stages()
        )
    );
}

#[tokio::test]
async fn failing_handler_skips_post_processor_and_send_returns_handler_error() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_post_processor(QuoteArchive {
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(RetiredItems {
            journal: Rc::clone(&journal),
        }))
        .build();

    let price = mediator.send(QuotePrice { sku: 7 }).await;

    assert_eq!(
        (price, journal.borrow().clone()),
        (
            Err(PriceError::Discontinued),
            vec!["Artikel 7 ausgelaufen".to_owned()]
        )
    );
}

#[tokio::test]
async fn same_pre_processor_type_registered_twice_runs_once_per_send() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_pre_processor(InquiryLog {
            journal: Rc::clone(&journal),
        })
        .open_request_pre_processor(InquiryLog {
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(PriceList {
            journal: Rc::clone(&journal),
        }))
        .build();

    let _price = mediator.send(QuotePrice { sku: 7 }).await;

    assert_eq!(
        journal.borrow().clone(),
        vec![
            "Anfrage für 7 eingegangen".to_owned(),
            "Preis berechnet für 7".to_owned()
        ]
    );
}

#[tokio::test]
async fn failing_pre_processor_skips_handler_and_send_returns_its_error() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_pre_processor(RetiredFilter {
            retired: 7,
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(PriceList {
            journal: Rc::clone(&journal),
        }))
        .build();

    let price = mediator.send(QuotePrice { sku: 7 }).await;

    assert_eq!(
        (price, journal.borrow().clone()),
        (
            Err(PriceError::Discontinued),
            vec!["Anfrage für 7 abgelehnt".to_owned()]
        )
    );
}

#[tokio::test]
async fn failing_post_processor_replaces_handler_response_with_its_error() {
    let journal = Journal::default();
    let mediator = Mediator::builder()
        .open_request_post_processor(PriceCeiling {
            max: 1000,
            journal: Rc::clone(&journal),
        })
        .register(Handler::instance(PriceList {
            journal: Rc::clone(&journal),
        }))
        .build();

    let price = mediator.send(QuotePrice { sku: 7 }).await;

    assert_eq!(
        (price, journal.borrow().clone()),
        (
            Err(PriceError::AboveCeiling),
            vec![
                "Preis berechnet für 7".to_owned(),
                "Preis 1999 über Obergrenze 1000".to_owned()
            ]
        )
    );
}
