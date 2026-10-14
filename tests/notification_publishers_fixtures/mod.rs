use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

use mediatr_rs::{
    Mediator, Notification, NotificationHandler, NotificationPublisher, Publisher, Request,
    RequestHandler, SendError, Sender, WeakMediator,
};

/// Ereignis: Das Paket mit dieser Sendungsnummer ist angekommen.
pub struct ParcelArrived {
    pub tracking: u32,
}

impl Notification for ParcelArrived {
    type Error = SendError;
}

/// Lagert jedes angekommene Paket ein und schreibt das ins Protokoll.
pub struct Stockroom {
    pub journal: Rc<RefCell<Vec<String>>>,
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
pub struct Recipient {
    pub journal: Rc<RefCell<Vec<String>>>,
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
pub struct NewestFirst;

impl NotificationPublisher for NewestFirst {
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

/// Ereignis: Das Paket mit dieser Sendungsnummer soll zugestellt werden.
pub struct DeliveryDue {
    pub tracking: u32,
}

impl Notification for DeliveryDue {
    type Error = SendError;
}

/// Meldet für jedes angekommene Paket über seinen Publisher, dass es zugestellt werden soll.
pub struct Dispatcher {
    pub publisher: Mediator<NewestFirst>,
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
pub struct CourierPlanner {
    pub journal: Rc<RefCell<Vec<String>>>,
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
pub struct DoorNotice {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<DeliveryDue> for DoorNotice {
    async fn handle(&self, event: &DeliveryDue) -> Result<(), SendError> {
        self.journal
            .borrow_mut()
            .push(format!("Tür-Hinweis {}", event.tracking));
        Ok(())
    }
}

/// Zentrale, die für jedes angekommene Paket über ihren Publisher meldet, dass es zugestellt werden soll; es gibt sie nur einmal.
pub struct DispatchCenter {
    pub publisher: WeakMediator<NewestFirst>,
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

/// Fragt nach, wo das Paket mit dieser Sendungsnummer gerade ist.
pub struct TrackParcel {
    pub tracking: u32,
}

impl Request for TrackParcel {
    type Response = String;
    type Error = SendError;
}

/// Kennt den Ort jedes Pakets im Lager.
pub struct TrackingDesk;

impl RequestHandler<TrackParcel> for TrackingDesk {
    async fn handle(&self, request: TrackParcel) -> Result<String, SendError> {
        Ok(format!("Paket {} liegt im Lager", request.tracking))
    }
}

/// Fragt für jedes angekommene Paket über seinen Sender nach, wo es liegt, und schreibt die Antwort ins Protokoll; es gibt ihn nur einmal.
pub struct ArrivalClerk {
    pub sender: WeakMediator<NewestFirst>,
    pub journal: Rc<RefCell<Vec<String>>>,
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

/// Zentrale mit Tag- und Nachtschicht: Jede Schicht meldet über ihren eigenen Publisher ein Paket zur Zustellung, die Nachtschicht das nächste; es gibt sie nur einmal.
pub struct ShiftCenter {
    pub day: WeakMediator<NewestFirst>,
    pub night: WeakMediator<NewestFirst>,
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

/// Entlädt jedes angekommene Paket vom Lastwagen; zwischen Beginn und Ende wartet es auf den Gabelstapler.
pub struct Unloading {
    pub journal: Rc<RefCell<Vec<String>>>,
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
pub struct Scanning {
    pub journal: Rc<RefCell<Vec<String>>>,
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

/// Ereignis: Das Paket mit dieser Sendungsnummer wurde am Wareneingang geprüft.
pub struct ParcelInspected {
    pub tracking: u32,
}

/// Warum ein Handler ein geprüftes Paket nicht annehmen kann.
#[derive(Debug, PartialEq)]
pub enum InspectionError {
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
pub struct DamageCheck;

impl NotificationHandler<ParcelInspected> for DamageCheck {
    async fn handle(&self, event: &ParcelInspected) -> Result<(), InspectionError> {
        Err(InspectionError::Damaged(event.tracking))
    }
}

/// Wiegt jedes geprüfte Paket und schreibt das ins Protokoll.
pub struct Weighing {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl NotificationHandler<ParcelInspected> for Weighing {
    async fn handle(&self, event: &ParcelInspected) -> Result<(), InspectionError> {
        self.journal
            .borrow_mut()
            .push(format!("gewogen {}", event.tracking));
        Ok(())
    }
}

/// Hält jedes geprüfte Paket beim Zoll fest.
pub struct CustomsHold;

impl NotificationHandler<ParcelInspected> for CustomsHold {
    async fn handle(&self, event: &ParcelInspected) -> Result<(), InspectionError> {
        Err(InspectionError::Held(event.tracking))
    }
}

/// Prüft jedes Paket gründlich und meldet es danach als beschädigt; zwischendurch wartet sie auf den Prüfstand.
pub struct ThoroughDamageCheck;

impl NotificationHandler<ParcelInspected> for ThoroughDamageCheck {
    async fn handle(&self, event: &ParcelInspected) -> Result<(), InspectionError> {
        tokio::task::yield_now().await;
        Err(InspectionError::Damaged(event.tracking))
    }
}
