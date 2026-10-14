//! Tests für den Abschnitt „Pipeline — Vor- und Nachverarbeiter“ in `TESTLIST.md`.

use std::cell::RefCell;
use std::rc::Rc;

use mediatr_rs::{Handler, Mediator, Sender};

mod processors_fixtures;

use processors_fixtures::{
    InquiryLog, PriceList, QuoteArchive, QuotePrice, checked_and_exported, price_check,
    price_export,
};

#[tokio::test]
async fn pre_processor_runs_before_handler_with_request_and_send_returns_handler_response() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .pre_processor(InquiryLog {
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
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .post_processor(QuoteArchive {
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
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .pre_processor(price_check::<1>(&journal))
        .pre_processor_for::<QuotePrice>(price_check::<2>(&journal))
        .pre_processor(price_check::<3>(&journal))
        .pre_processor_for::<QuotePrice>(price_check::<4>(&journal))
        .post_processor_for::<QuotePrice>(price_export::<1>(&journal))
        .post_processor(price_export::<2>(&journal))
        .post_processor_for::<QuotePrice>(price_export::<3>(&journal))
        .post_processor(price_export::<4>(&journal))
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
