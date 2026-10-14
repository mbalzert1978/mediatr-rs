//! Tests für den Abschnitt „Pipeline — Vor- und Nachverarbeiter“ in `TESTLIST.md`.

use std::cell::RefCell;
use std::rc::Rc;

use mediatr_rs::{Handler, Mediator, Sender};

mod processors_fixtures;

use processors_fixtures::{InquiryLog, PriceList, QuoteArchive, QuotePrice};

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
