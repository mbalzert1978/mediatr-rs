//! Tests für den Abschnitt „Pipeline — Behaviors“ in `TESTLIST.md`.

use std::cell::Cell;
use std::rc::Rc;

use mediatr_rs::{
    Handler, Mediator, Next, PipelineBehavior, Request, RequestHandler, SendError, Sender,
};

/// Fragt den Lagerbestand des Artikels mit dieser Nummer ab.
struct LookupStock {
    sku: u32,
}

/// Warum eine Bestandsabfrage scheitert.
#[derive(Debug, PartialEq)]
enum StockError {
    Timeout,
    Undeliverable(SendError),
}

impl From<SendError> for StockError {
    fn from(error: SendError) -> Self {
        Self::Undeliverable(error)
    }
}

impl Request for LookupStock {
    type Response = u32;
    type Error = StockError;
}

/// Führt den Lagerbestand; der Lagerserver antwortet auf die erste Abfrage zu spät.
struct SlowStockLedger {
    calls: Rc<Cell<u32>>,
}

impl RequestHandler<LookupStock> for SlowStockLedger {
    async fn handle(&self, _request: LookupStock) -> Result<u32, StockError> {
        let call = self.calls.get().saturating_add(1);
        self.calls.set(call);
        if call == 1 {
            return Err(StockError::Timeout);
        }
        Ok(12)
    }
}

/// Wiederholt eine Bestandsabfrage einmal, wenn sie scheitert.
struct RetryOnce;

impl PipelineBehavior<LookupStock> for RetryOnce {
    async fn handle(
        &self,
        request: LookupStock,
        next: Next<LookupStock>,
    ) -> Result<u32, StockError> {
        let sku = request.sku;
        match next.clone().run(request).await {
            Ok(stock) => Ok(stock),
            Err(_) => next.run(LookupStock { sku }).await,
        }
    }
}

#[tokio::test]
async fn behavior_retries_cloned_next_after_handler_error_and_returns_second_response() {
    let calls = Rc::new(Cell::new(0));
    let mediator = Mediator::builder()
        .behavior(Handler::instance(RetryOnce))
        .register(Handler::instance(SlowStockLedger {
            calls: Rc::clone(&calls),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!((stock, calls.get()), (Ok(12), 2));
}
