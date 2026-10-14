//! Tests für den Abschnitt „Notification — Publish-Strategie“ in `TESTLIST.md`.

use std::cell::{OnceCell, RefCell};
use std::future::Future;
use std::rc::Rc;

use mediatr_rs::{
    Concurrent, Handler, Mediator, MediatorBuilder, Notification, NotificationHandler,
    PublishStrategy, Publisher, Request, RequestHandler, SendError, Sender, WeakMediator,
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

#[tokio::test]
async fn custom_strategy_without_debug_mediator_weak_mediator_and_builder_debug_names_strategy() {
    let weak_output = Rc::new(RefCell::new(String::new()));
    let seen = Rc::clone(&weak_output);
    let builder = MediatorBuilder::<NewestFirst>::default().subscribe(Handler::singleton(
        move |publisher: WeakMediator<NewestFirst>| {
            *seen.borrow_mut() = format!("{publisher:?}");
            DispatchCenter { publisher }
        },
    ));
    let builder_output = format!("{builder:?}");
    let mediator = builder.build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    let outputs = [
        format!("{mediator:?}"),
        weak_output.borrow().clone(),
        builder_output,
    ];
    assert!(
        outputs.iter().all(|output| output.contains("NewestFirst")),
        "{outputs:?}"
    );
}

#[tokio::test]
async fn custom_strategy_without_clone_builder_clone_builds_mediators_reaching_same_handlers() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let builder = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::instance(CourierPlanner {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(DoorNotice {
            journal: Rc::clone(&journal),
        }));
    let copy = builder.clone();

    builder
        .build()
        .publish(DeliveryDue { tracking: 7 })
        .await
        .unwrap();
    copy.build()
        .publish(DeliveryDue { tracking: 8 })
        .await
        .unwrap();

    assert_eq!(
        *journal.borrow(),
        [
            "Tür-Hinweis 7",
            "Kurier geplant 7",
            "Tür-Hinweis 8",
            "Kurier geplant 8"
        ]
    );
}

/// Fragt nach, wo das Paket mit dieser Sendungsnummer gerade ist.
struct TrackParcel {
    tracking: u32,
}

impl Request for TrackParcel {
    type Response = String;
    type Error = SendError;
}

/// Kennt den Ort jedes Pakets im Lager.
struct TrackingDesk;

impl RequestHandler<TrackParcel> for TrackingDesk {
    async fn handle(&self, request: TrackParcel) -> Result<String, SendError> {
        Ok(format!("Paket {} liegt im Lager", request.tracking))
    }
}

#[tokio::test]
async fn custom_strategy_send_returns_handler_response() {
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .register(Handler::instance(TrackingDesk))
        .build();

    let place = mediator.send(TrackParcel { tracking: 7 }).await;

    assert_eq!(place, Ok("Paket 7 liegt im Lager".to_owned()));
}

/// Fragt für jedes angekommene Paket über seinen Sender nach, wo es liegt, und schreibt die Antwort ins Protokoll; es gibt ihn nur einmal.
struct ArrivalClerk {
    sender: WeakMediator<NewestFirst>,
    journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<ParcelArrived> for ArrivalClerk {
    async fn handle(&self, event: &ParcelArrived) -> Result<(), SendError> {
        let place = self
            .sender
            .send(TrackParcel {
                tracking: event.tracking,
            })
            .await?;
        self.journal.borrow_mut().push(place);
        Ok(())
    }
}

#[tokio::test]
async fn custom_strategy_singleton_handler_sends_through_received_weak_mediator() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let clerk_journal = Rc::clone(&journal);
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .register(Handler::instance(TrackingDesk))
        .subscribe(Handler::singleton(move |sender| ArrivalClerk {
            sender,
            journal: Rc::clone(&clerk_journal),
        }))
        .build();

    let published = mediator.publish(ParcelArrived { tracking: 7 }).await;

    assert_eq!(
        (published, journal.borrow().clone()),
        (Ok(()), vec!["Paket 7 liegt im Lager".to_owned()])
    );
}

/// Zentrale mit Tag- und Nachtschicht: Jede Schicht meldet über ihren eigenen Publisher ein Paket zur Zustellung, die Nachtschicht das nächste; es gibt sie nur einmal.
struct ShiftCenter {
    day: WeakMediator<NewestFirst>,
    night: WeakMediator<NewestFirst>,
}

impl NotificationHandler<ParcelArrived> for ShiftCenter {
    async fn handle(&self, event: &ParcelArrived) -> Result<(), SendError> {
        self.day
            .publish(DeliveryDue {
                tracking: event.tracking,
            })
            .await?;
        self.night
            .publish(DeliveryDue {
                tracking: event.tracking.saturating_add(1),
            })
            .await
    }
}

#[tokio::test]
async fn custom_strategy_singleton_handler_publishes_through_weak_mediator_and_its_clone() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::singleton(
            |publisher: WeakMediator<NewestFirst>| ShiftCenter {
                night: publisher.clone(),
                day: publisher,
            },
        ))
        .subscribe(Handler::instance(CourierPlanner {
            journal: Rc::clone(&journal),
        }))
        .build();

    let published = mediator.publish(ParcelArrived { tracking: 7 }).await;

    assert_eq!(
        (published, journal.borrow().clone()),
        (
            Ok(()),
            vec!["Kurier geplant 7".to_owned(), "Kurier geplant 8".to_owned()]
        )
    );
}

#[tokio::test]
async fn custom_strategy_singleton_handler_publishing_after_mediator_dropped_gets_mediator_dropped()
{
    let handed_out = Rc::new(OnceCell::new());
    let keep = Rc::clone(&handed_out);
    let mediator = MediatorBuilder::<NewestFirst>::default()
        .subscribe(Handler::singleton(
            move |publisher: WeakMediator<NewestFirst>| {
                keep.get_or_init(|| publisher.clone());
                DispatchCenter { publisher }
            },
        ))
        .build();
    mediator
        .publish(ParcelArrived { tracking: 6 })
        .await
        .unwrap();
    let center = DispatchCenter {
        publisher: handed_out.get().unwrap().clone(),
    };
    drop(mediator);

    let result = center.handle(&ParcelArrived { tracking: 7 }).await;

    assert_eq!(result, Err(SendError::MediatorDropped));
}

/// Entlädt jedes angekommene Paket vom Lastwagen; zwischen Beginn und Ende wartet es auf den Gabelstapler.
struct Unloading {
    journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<ParcelArrived> for Unloading {
    async fn handle(&self, event: &ParcelArrived) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Entladen beginnt {}", event.tracking));
        tokio::task::yield_now().await;
        self.journal
            .borrow_mut()
            .push(format!("Entladen fertig {}", event.tracking));
        Ok(())
    }
}

/// Scannt jedes angekommene Paket; zwischen Beginn und Ende wartet es auf den Scanner.
struct Scanning {
    journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<ParcelArrived> for Scanning {
    async fn handle(&self, event: &ParcelArrived) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Scannen beginnt {}", event.tracking));
        tokio::task::yield_now().await;
        self.journal
            .borrow_mut()
            .push(format!("Scannen fertig {}", event.tracking));
        Ok(())
    }
}

#[tokio::test]
async fn concurrent_strategy_starts_second_waiting_handler_before_first_finished() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<Concurrent>::default()
        .subscribe(Handler::instance(Unloading {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(Scanning {
            journal: Rc::clone(&journal),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    assert_eq!(
        *journal.borrow(),
        [
            "Entladen beginnt 7",
            "Scannen beginnt 7",
            "Entladen fertig 7",
            "Scannen fertig 7"
        ]
    );
}

/// Ereignis: Das Paket mit dieser Sendungsnummer wurde am Wareneingang geprüft.
struct ParcelInspected {
    tracking: u32,
}

/// Warum ein Handler ein geprüftes Paket nicht annehmen kann.
#[derive(Debug, PartialEq)]
enum InspectionError {
    Damaged(u32),
    Held(u32),
    Undeliverable(SendError),
}

impl From<SendError> for InspectionError {
    fn from(error: SendError) -> Self {
        Self::Undeliverable(error)
    }
}

impl Notification for ParcelInspected {
    type Error = InspectionError;
}

/// Meldet jedes geprüfte Paket als beschädigt.
struct DamageCheck;

impl NotificationHandler<ParcelInspected> for DamageCheck {
    async fn handle(&self, event: &ParcelInspected) -> Result<(), InspectionError> {
        Err(InspectionError::Damaged(event.tracking))
    }
}

/// Wiegt jedes geprüfte Paket und schreibt das ins Protokoll.
struct Weighing {
    journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<ParcelInspected> for Weighing {
    async fn handle(&self, event: &ParcelInspected) -> Result<(), InspectionError> {
        self.journal
            .borrow_mut()
            .push(format!("gewogen {}", event.tracking));
        Ok(())
    }
}

#[tokio::test]
async fn concurrent_strategy_runs_second_handler_after_first_fails_and_returns_first_error() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<Concurrent>::default()
        .subscribe(Handler::instance(DamageCheck))
        .subscribe(Handler::instance(Weighing {
            journal: Rc::clone(&journal),
        }))
        .build();

    let published = mediator.publish(ParcelInspected { tracking: 7 }).await;

    assert_eq!(
        (published, journal.borrow().clone()),
        (
            Err(InspectionError::Damaged(7)),
            vec!["gewogen 7".to_owned()]
        )
    );
}

#[tokio::test]
async fn concurrent_strategy_runs_each_of_three_handlers_once_and_returns_ok() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<Concurrent>::default()
        .subscribe(Handler::instance(Stockroom {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(Recipient {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(Scanning {
            journal: Rc::clone(&journal),
        }))
        .build();

    let published = mediator.publish(ParcelArrived { tracking: 7 }).await;

    let mut entries = journal.borrow().clone();
    entries.sort();
    assert_eq!(
        (published, entries),
        (
            Ok(()),
            vec![
                "Empfänger benachrichtigt 7".to_owned(),
                "Scannen beginnt 7".to_owned(),
                "Scannen fertig 7".to_owned(),
                "eingelagert 7".to_owned()
            ]
        )
    );
}

/// Hält jedes geprüfte Paket beim Zoll fest.
struct CustomsHold;

impl NotificationHandler<ParcelInspected> for CustomsHold {
    async fn handle(&self, event: &ParcelInspected) -> Result<(), InspectionError> {
        Err(InspectionError::Held(event.tracking))
    }
}

#[tokio::test]
async fn concurrent_strategy_both_handlers_fail_returns_error_of_first_registered() {
    let mediator = MediatorBuilder::<Concurrent>::default()
        .subscribe(Handler::instance(DamageCheck))
        .subscribe(Handler::instance(CustomsHold))
        .build();

    let published = mediator.publish(ParcelInspected { tracking: 7 }).await;

    assert_eq!(published, Err(InspectionError::Damaged(7)));
}

#[tokio::test]
async fn concurrent_strategy_only_second_handler_fails_runs_first_and_returns_second_error() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<Concurrent>::default()
        .subscribe(Handler::instance(Weighing {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(DamageCheck))
        .build();

    let published = mediator.publish(ParcelInspected { tracking: 7 }).await;

    assert_eq!(
        (published, journal.borrow().clone()),
        (
            Err(InspectionError::Damaged(7)),
            vec!["gewogen 7".to_owned()]
        )
    );
}

/// Prüft jedes Paket gründlich und meldet es danach als beschädigt; zwischendurch wartet sie auf den Prüfstand.
struct ThoroughDamageCheck;

impl NotificationHandler<ParcelInspected> for ThoroughDamageCheck {
    async fn handle(&self, event: &ParcelInspected) -> Result<(), InspectionError> {
        tokio::task::yield_now().await;
        Err(InspectionError::Damaged(event.tracking))
    }
}

#[tokio::test]
async fn concurrent_strategy_both_fail_first_after_await_returns_error_of_first_registered() {
    let mediator = MediatorBuilder::<Concurrent>::default()
        .subscribe(Handler::instance(ThoroughDamageCheck))
        .subscribe(Handler::instance(CustomsHold))
        .build();

    let published = mediator.publish(ParcelInspected { tracking: 7 }).await;

    assert_eq!(published, Err(InspectionError::Damaged(7)));
}

#[tokio::test]
async fn concurrent_strategy_equally_waiting_handlers_finish_in_registration_order() {
    let journal = Rc::new(RefCell::new(Vec::new()));
    let mediator = MediatorBuilder::<Concurrent>::default()
        .subscribe(Handler::instance(Unloading {
            journal: Rc::clone(&journal),
        }))
        .subscribe(Handler::instance(Scanning {
            journal: Rc::clone(&journal),
        }))
        .build();

    mediator
        .publish(ParcelArrived { tracking: 7 })
        .await
        .unwrap();

    let finished: Vec<_> = journal
        .borrow()
        .iter()
        .filter(|entry| entry.contains("fertig"))
        .cloned()
        .collect();
    assert_eq!(finished, ["Entladen fertig 7", "Scannen fertig 7"]);
}
