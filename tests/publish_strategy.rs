//! Tests für den Abschnitt „Notification — Publish-Strategie“ in `TESTLIST.md`.

use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

use mediatr_rs::{
    Handler, MediatorBuilder, Notification, NotificationHandler, PublishStrategy, Publisher,
    SendError,
};

/// Ereignis: Das Paket mit dieser Sendungsnummer ist angekommen.
struct ParcelArrived {
    tracking: u32,
}

impl Notification for ParcelArrived {
    type Error = SendError;
}

/// Lagert jedes angekommene Paket ein und schreibt das ins Protokoll.
struct Stockroom {
    journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<ParcelArrived> for Stockroom {
    async fn handle(&self, event: &ParcelArrived) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("eingelagert {}", event.tracking));
        Ok(())
    }
}

/// Benachrichtigt den Empfänger jedes angekommenen Pakets und schreibt das ins Protokoll.
struct Recipient {
    journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<ParcelArrived> for Recipient {
    async fn handle(&self, event: &ParcelArrived) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Empfänger benachrichtigt {}", event.tracking));
        Ok(())
    }
}

/// Ruft die Handler nacheinander auf, den zuletzt registrierten zuerst.
struct NewestFirst;

impl PublishStrategy for NewestFirst {
    async fn publish<E, F>(calls: Vec<F>) -> Result<(), E>
    where
        F: Future<Output = Result<(), E>>,
    {
        for call in calls.into_iter().rev() {
            call.await?;
        }
        Ok(())
    }
}

#[tokio::test]
async fn custom_strategy_runs_both_handlers_in_its_own_order() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::instance(Stockroom {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(Recipient {
            journal: Rc::clone(&journal),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(
        *journal.borrow(),
        ["Empfänger benachrichtigt 7", "eingelagert 7"]
    );
}
