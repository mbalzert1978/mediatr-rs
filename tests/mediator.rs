//! Tests für den Abschnitt „Mediator — Fassade und DI“ in `TESTLIST.md`.

use mediatr_rs::{Mediator, MissingHandler, Request, RequestHandler, Sender};

struct Inner;

impl Request for Inner {
    type Response = String;
    type Error = MissingHandler;
}

struct Outer;

impl Request for Outer {
    type Response = String;
    type Error = MissingHandler;
}

struct OuterHandler {
    sender: Sender,
}

impl RequestHandler<Outer> for OuterHandler {
    async fn handle(&self, _request: Outer) -> Result<String, MissingHandler> {
        let inner = self.sender.send(Inner).await?;
        Ok(format!("außen({inner})"))
    }
}

#[tokio::test]
async fn handler_sends_inner_request_through_injected_sender() {
    let mediator = Mediator::builder()
        .register(async |_: Inner| Ok("innen".to_owned()))
        .transient(|sender| OuterHandler { sender })
        .build();

    let response = mediator.send(Outer).await.unwrap();

    assert_eq!(response, "außen(innen)");
}
