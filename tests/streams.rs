//! Tests für den Abschnitt „Stream-Request — Stream anfordern“ in `TESTLIST.md`.

use futures_util::StreamExt;
use mediatr_rs::{Handler, Mediator, SendError, Sender, StreamSender};

mod streams_fixtures;

use streams_fixtures::{ArrivalBoard, CountArrivals, GuestBook, ListArrivals};

#[tokio::test]
async fn create_stream_yields_exactly_handler_items_then_ends() {
    let mediator = Mediator::builder()
        .register_stream(Handler::instance(GuestBook {
            guests: vec!["Anna".to_owned(), "Ben".to_owned(), "Clara".to_owned()],
        }))
        .build();

    let arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 3 })
        .collect()
        .await;

    assert_eq!(
        arrivals,
        vec![
            Ok("Anna am Tag 3".to_owned()),
            Ok("Ben am Tag 3".to_owned()),
            Ok("Clara am Tag 3".to_owned())
        ]
    );
}

#[tokio::test]
async fn create_stream_without_handler_yields_no_handler_error() {
    let mediator = Mediator::builder().build();

    let arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 3 })
        .collect()
        .await;

    assert_eq!(arrivals, vec![Err(SendError::NoHandler)]);
}

#[tokio::test]
async fn handler_requests_stream_through_constructor_sender_and_gets_handler_items() {
    let mediator = Mediator::builder()
        .register_stream(Handler::instance(GuestBook {
            guests: vec!["Anna".to_owned(), "Ben".to_owned(), "Clara".to_owned()],
        }))
        .register(Handler::transient(|mediator| ArrivalBoard {
            arrivals: mediator,
        }))
        .build();

    let count = mediator.send(CountArrivals { day: 3 }).await;

    assert_eq!(count, Ok(3));
}

#[tokio::test]
async fn register_stream_twice_keeps_last_stream_handler() {
    let mediator = Mediator::builder()
        .register_stream(Handler::instance(GuestBook {
            guests: vec!["Anna".to_owned()],
        }))
        .register_stream(Handler::instance(GuestBook {
            guests: vec!["Dora".to_owned(), "Emil".to_owned()],
        }))
        .build();

    let arrivals: Vec<_> = mediator
        .create_stream(ListArrivals { day: 5 })
        .collect()
        .await;

    assert_eq!(
        arrivals,
        vec![
            Ok("Dora am Tag 5".to_owned()),
            Ok("Emil am Tag 5".to_owned())
        ]
    );
}
