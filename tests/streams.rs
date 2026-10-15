//! Tests für den Abschnitt „Stream-Request — Stream anfordern“ in `TESTLIST.md`.

use futures_util::StreamExt;
use mediatr_rs::{Handler, Mediator, StreamSender};

mod streams_fixtures;

use streams_fixtures::{GuestBook, ListArrivals};

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
