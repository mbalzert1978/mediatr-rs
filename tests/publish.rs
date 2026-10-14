//! Tests für den Abschnitt „Notification — `publish`“ in `TESTLIST.md`.

use std::cell::RefCell;
use std::rc::Rc;

use mediatr_rs::{Mediator, Notification, NotificationHandler, Publisher, SendError};

/// Ereignis: Die Bestellung mit dieser Nummer ist eingegangen.
struct OrderPlaced {
    order_id: u32,
}

impl Notification for OrderPlaced {
    type Error = SendError;
}

/// Druckt für jede eingegangene Bestellung ein Versandetikett mit ihrer Nummer.
struct ShippingDesk {
    labels: Rc<RefCell<Vec<u32>>>,
}

impl NotificationHandler<OrderPlaced> for ShippingDesk {
    async fn handle(&self, event: &OrderPlaced) -> Result<(), SendError> {
        self.labels.borrow_mut().push(event.order_id);
        Ok(())
    }
}

#[tokio::test]
async fn publish_runs_registered_handler() {
    let labels = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .subscribe(ShippingDesk {
            labels: Rc::clone(&labels),
        })
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(*labels.borrow(), [42]);
}

#[tokio::test]
async fn publish_without_handler_returns_ok() {
    let mediator = Mediator::builder().build();

    let result = mediator.publish(OrderPlaced { order_id: 42 }).await;

    assert!(matches!(result, Ok(())));
}

/// Reserviert für jede eingegangene Bestellung die Ware und schreibt das ins Protokoll.
struct Warehouse {
    journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<OrderPlaced> for Warehouse {
    async fn handle(&self, event: &OrderPlaced) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Ware reserviert für {}", event.order_id));
        Ok(())
    }
}

/// Stellt für jede eingegangene Bestellung eine Rechnung aus und schreibt das ins Protokoll.
struct Billing {
    journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<OrderPlaced> for Billing {
    async fn handle(&self, event: &OrderPlaced) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Rechnung für {}", event.order_id));
        Ok(())
    }
}

#[tokio::test]
async fn publish_runs_both_handlers_in_registration_order() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = Mediator::builder()
        .subscribe(Warehouse {
            journal: Rc::clone(&journal),
        })
        .subscribe(Billing {
            journal: Rc::clone(&journal),
        })
        .build();

    mediator
        .publish(OrderPlaced { order_id: 42 })
        .await
        .unwrap();

    assert_eq!(
        *journal.borrow(),
        ["Ware reserviert für 42", "Rechnung für 42"]
    );
}
