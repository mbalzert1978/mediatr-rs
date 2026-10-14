//! Tests für den Abschnitt „Pipeline — Behaviors“ in `TESTLIST.md`.

use std::cell::{Cell, RefCell};
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
    let mediator = Mediator::with_behaviors((RetryOnce,))
        .register(Handler::instance(SlowStockLedger {
            calls: Rc::clone(&calls),
        }))
        .build();

    let stock = mediator.send(LookupStock { sku: 4 }).await;

    assert_eq!((stock, calls.get()), (Ok(12), 2));
}

/// Führt den Lagerbestand und schreibt jede Abfrage ins Protokoll.
struct StockLedger {
    journal: Rc<RefCell<Vec<String>>>,
}

impl RequestHandler<LookupStock> for StockLedger {
    async fn handle(&self, request: LookupStock) -> Result<u32, StockError> {
        self.journal
            .borrow_mut()
            .push(format!("Bestand gelesen für {}", request.sku));
        Ok(12)
    }
}

/// Schreibt vor und nach jedem Request einen Eintrag ins Prüfprotokoll.
struct AuditTrail {
    journal: Rc<RefCell<Vec<String>>>,
}

impl<R: Request> PipelineBehavior<R> for AuditTrail {
    async fn handle(&self, request: R, next: Next<R>) -> Result<R::Response, R::Error> {
        self.journal.borrow_mut().push("Prüfung beginnt".to_owned());
        let response = next.run(request).await;
        self.journal.borrow_mut().push("Prüfung endet".to_owned());
        response
    }
}

/// Misst jeden Request und schreibt Start und Ende ins Protokoll.
struct Stopwatch {
    journal: Rc<RefCell<Vec<String>>>,
}

impl<R: Request> PipelineBehavior<R> for Stopwatch {
    async fn handle(&self, request: R, next: Next<R>) -> Result<R::Response, R::Error> {
        self.journal.borrow_mut().push("Messung startet".to_owned());
        let response = next.run(request).await;
        self.journal.borrow_mut().push("Messung stoppt".to_owned());
        response
    }
}

/// Prüfprotokoll und Stoppuhr, die beide in `journal` schreiben; die Prüfung liegt außen.
fn audit_and_stopwatch(journal: &Rc<RefCell<Vec<String>>>) -> (AuditTrail, Stopwatch) {
    (
        AuditTrail {
            journal: Rc::clone(journal),
        },
        Stopwatch {
            journal: Rc::clone(journal),
        },
    )
}

/// Was [`audit_and_stopwatch`] um den Protokolleintrag `handled` eines Handlers herum ins Protokoll schreibt.
fn audited_and_timed(handled: &str) -> Vec<String> {
    [
        "Prüfung beginnt",
        "Messung startet",
        handled,
        "Messung stoppt",
        "Prüfung endet",
    ]
    .map(str::to_owned)
    .to_vec()
}

#[tokio::test]
async fn two_behaviors_wrap_handler_first_registered_outermost() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::with_behaviors(audit_and_stopwatch(&journal))
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

/// Bucht den Wareneingang des Artikels mit dieser Nummer; die Buchung hat keine Antwort.
struct BookReceipt {
    sku: u32,
}

impl Request for BookReceipt {
    type Response = ();
    type Error = StockError;
}

/// Bucht jeden Wareneingang und schreibt das ins Protokoll.
struct ReceiptBook {
    journal: Rc<RefCell<Vec<String>>>,
}

impl RequestHandler<BookReceipt> for ReceiptBook {
    async fn handle(&self, request: BookReceipt) -> Result<(), StockError> {
        self.journal
            .borrow_mut()
            .push(format!("Eingang gebucht für {}", request.sku));
        Ok(())
    }
}

#[tokio::test]
async fn two_behaviors_wrap_unit_response_handler_first_registered_outermost() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::with_behaviors(audit_and_stopwatch(&journal))
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
    let mediator = Mediator::with_behaviors(audit_and_stopwatch(&journal))
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
