use std::cell::RefCell;
use std::rc::Rc;

use mediatr_rs::{
    Next, PipelineBehavior, Request, RequestErrorAction, RequestErrorHandler, RequestHandler,
    SendError,
};

/// Bucht das Hotelzimmer mit dieser Nummer und liefert die Buchungsnummer.
#[derive(Clone)]
pub struct BookRoom {
    pub room: u32,
}

/// Warum eine Buchung scheitert.
#[derive(Debug, PartialEq)]
pub enum BookingError {
    FullyBooked,
    WaitingListClosed,
    PaymentDeclined,
    AlarmFailed,
    Undeliverable(SendError),
}

impl From<SendError> for BookingError {
    fn from(error: SendError) -> Self {
        Self::Undeliverable(error)
    }
}

impl Request for BookRoom {
    type Response = u64;
    type Error = BookingError;
}

/// Ein ausgebuchtes Hotel: Es schreibt jede Anfrage ins Protokoll und lehnt sie ab.
pub struct FullHotel {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestHandler<BookRoom> for FullHotel {
    async fn handle(&self, request: BookRoom) -> Result<u64, BookingError> {
        self.journal
            .borrow_mut()
            .push(format!("Zimmer {} ausgebucht", request.room));
        Err(BookingError::FullyBooked)
    }
}

/// Schlägt bei jeder gescheiterten Buchung Alarm im Protokoll.
pub struct BookingAlert {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestErrorAction<BookRoom> for BookingAlert {
    async fn execute(&self, request: &BookRoom, error: &BookingError) -> Result<(), BookingError> {
        self.journal.borrow_mut().push(format!(
            "Alarm: Buchung für Zimmer {} gescheitert: {error:?}",
            request.room
        ));
        Ok(())
    }
}

/// Informiert den Gast über jede gescheiterte Buchung und schreibt das ins Protokoll.
pub struct GuestNotice {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestErrorAction<BookRoom> for GuestNotice {
    async fn execute(&self, request: &BookRoom, _error: &BookingError) -> Result<(), BookingError> {
        self.journal.borrow_mut().push(format!(
            "Gast informiert: Zimmer {} nicht verfügbar",
            request.room
        ));
        Ok(())
    }
}

/// Setzt jede gescheiterte Buchung auf die Warteliste und liefert die Wartelistennummer als Buchungsnummer.
pub struct WaitingList {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestErrorHandler<BookRoom> for WaitingList {
    async fn handle(&self, request: &BookRoom, _error: BookingError) -> Result<u64, BookingError> {
        self.journal
            .borrow_mut()
            .push(format!("Zimmer {} auf der Warteliste", request.room));
        Ok(9000)
    }
}

/// Bucht bei jeder gescheiterten Buchung ein Zimmer im Partnerhotel und liefert dessen Buchungsnummer.
pub struct PartnerHotel {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestErrorHandler<BookRoom> for PartnerHotel {
    async fn handle(&self, request: &BookRoom, _error: BookingError) -> Result<u64, BookingError> {
        self.journal
            .borrow_mut()
            .push(format!("Zimmer {} im Partnerhotel gebucht", request.room));
        Ok(7000)
    }
}

/// Prüft bei jeder gescheiterten Buchung, ob sich überbuchen lässt; das geht nie, darum reicht sie den Fehler weiter.
pub struct OverbookingCheck {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestErrorHandler<BookRoom> for OverbookingCheck {
    async fn handle(&self, request: &BookRoom, error: BookingError) -> Result<u64, BookingError> {
        self.journal
            .borrow_mut()
            .push(format!("Überbuchung für Zimmer {} abgelehnt", request.room));
        Err(error)
    }
}

/// Eine geschlossene Warteliste: Sie schreibt jede gescheiterte Buchung ins Protokoll und scheitert selbst.
pub struct ClosedWaitingList {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestErrorHandler<BookRoom> for ClosedWaitingList {
    async fn handle(&self, request: &BookRoom, _error: BookingError) -> Result<u64, BookingError> {
        self.journal.borrow_mut().push(format!(
            "Warteliste für Zimmer {} geschlossen",
            request.room
        ));
        Err(BookingError::WaitingListClosed)
    }
}

/// Lässt keine Buchung durch, weil jede Zahlung abgelehnt wird, und schreibt jede Ablehnung ins Protokoll.
pub struct PaymentGate {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl PipelineBehavior<BookRoom> for PaymentGate {
    async fn handle(&self, request: BookRoom, _next: Next<BookRoom>) -> Result<u64, BookingError> {
        self.journal
            .borrow_mut()
            .push(format!("Zahlung für Zimmer {} abgelehnt", request.room));
        Err(BookingError::PaymentDeclined)
    }
}

/// Eine kaputte Alarmanlage: Sie schreibt jede gescheiterte Buchung ins Protokoll und scheitert dann selbst.
pub struct BrokenAlarm {
    pub journal: Rc<RefCell<Vec<String>>>,
}

impl RequestErrorAction<BookRoom> for BrokenAlarm {
    async fn execute(&self, request: &BookRoom, _error: &BookingError) -> Result<(), BookingError> {
        self.journal
            .borrow_mut()
            .push(format!("Alarm für Zimmer {} ausgefallen", request.room));
        Err(BookingError::AlarmFailed)
    }
}
