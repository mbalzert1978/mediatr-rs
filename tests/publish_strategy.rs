//! Tests für den Abschnitt „Notification — Publish-Strategie“ in `TESTLIST.md`.

use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

use mediatr_rs::{
    Handler, Mediator, MediatorBuilder, Notification, NotificationHandler, PublishStrategy,
    Publisher, SendError, WeakMediator,
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

/// Ereignis: Das Paket mit dieser Sendungsnummer soll zugestellt werden.
struct DeliveryDue {
    tracking: u32,
}

impl Notification for DeliveryDue {
    type Error = SendError;
}

/// Meldet für jedes angekommene Paket über seinen Publisher, dass es zugestellt werden soll.
struct Dispatcher {
    publisher: Mediator<NewestFirst>,
}

impl NotificationHandler<ParcelArrived> for Dispatcher {
    async fn handle(&self, event: &ParcelArrived) -> Result<(), SendError> {
        self.publisher
            .publish(DeliveryDue {
                tracking: event.tracking,
            })
            .await
    }
}

/// Plant für jedes fällige Paket einen Kurier und schreibt das ins Protokoll.
struct CourierPlanner {
    journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<DeliveryDue> for CourierPlanner {
    async fn handle(&self, event: &DeliveryDue) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Kurier geplant {}", event.tracking));
        Ok(())
    }
}

/// Kündigt jedes fällige Paket an der Haustür an und schreibt das ins Protokoll.
struct DoorNotice {
    journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<DeliveryDue> for DoorNotice {
    async fn handle(&self, event: &DeliveryDue) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Tür-Hinweis {}", event.tracking));
        Ok(())
    }
}

#[tokio::test]
async fn custom_strategy_transient_handler_publishes_through_received_mediator_in_its_order() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::transient(|publisher| Dispatcher { publisher }))
        .subscribe(Handler::instance(CourierPlanner {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(DoorNotice {
            journal: Rc::clone(&journal),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(*journal.borrow(), ["Tür-Hinweis 7", "Kurier geplant 7"]);
}

/// Zentrale, die für jedes angekommene Paket über ihren Publisher meldet, dass es zugestellt werden soll; es gibt sie nur einmal.
struct DispatchCenter {
    publisher: WeakMediator<NewestFirst>,
}

impl NotificationHandler<ParcelArrived> for DispatchCenter {
    async fn handle(&self, event: &ParcelArrived) -> Result<(), SendError> {
        self.publisher
            .publish(DeliveryDue {
                tracking: event.tracking,
            })
            .await
    }
}

#[tokio::test]
async fn custom_strategy_singleton_handler_publishes_through_received_weak_mediator_in_its_order() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::singleton(|publisher| DispatchCenter { publisher }))
        .subscribe(Handler::instance(CourierPlanner {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(DoorNotice {
            journal: Rc::clone(&journal),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(*journal.borrow(), ["Tür-Hinweis 7", "Kurier geplant 7"]);
}

#[tokio::test]
async fn custom_strategy_without_clone_mediator_and_weak_mediator_clone_reaches_same_handlers() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::singleton(
            |publisher: WeakMediator<NewestFirst>| DispatchCenter {
                publisher: publisher.clone(),
            },
        ))
        .subscribe(Handler::instance(CourierPlanner {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(DoorNotice {
            journal: Rc::clone(&journal),
        }))
        .build();

    mediator
        .clone()
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(*journal.borrow(), ["Tür-Hinweis 7", "Kurier geplant 7"]);
}
