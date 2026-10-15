use mediatr_rs::{
    Next, PipelineBehavior, Request, RequestErrorAction, RequestErrorHandler, RequestHandler,
    RequestPostProcessor, RequestPreProcessor, SendError,
};

use crate::common::Journal;

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
    UnderRenovation,
    ConfirmationFailed,
    NotRefundable,
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
    pub journal: Journal,
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
    pub journal: Journal,
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
    pub journal: Journal,
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
    pub journal: Journal,
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
    pub journal: Journal,
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
    pub journal: Journal,
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
    pub journal: Journal,
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
    pub journal: Journal,
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
    pub journal: Journal,
}

impl RequestErrorAction<BookRoom> for BrokenAlarm {
    async fn execute(&self, request: &BookRoom, _error: &BookingError) -> Result<(), BookingError> {
        self.journal
            .borrow_mut()
            .push(format!("Alarm für Zimmer {} ausgefallen", request.room));
        Err(BookingError::AlarmFailed)
    }
}

/// Ein Hotel mit freien Zimmern: Es schreibt jede Buchung ins Protokoll und liefert die Buchungsnummer.
pub struct OpenHotel {
    pub journal: Journal,
}

impl RequestHandler<BookRoom> for OpenHotel {
    async fn handle(&self, request: BookRoom) -> Result<u64, BookingError> {
        self.journal
            .borrow_mut()
            .push(format!("Zimmer {} gebucht", request.room));
        Ok(4711)
    }
}

/// Prüft vor jeder Buchung, ob das Zimmer renoviert wird; das ist immer so, darum lehnt sie jede Buchung ab und schreibt das ins Protokoll.
pub struct RenovationCheck {
    pub journal: Journal,
}

impl RequestPreProcessor<BookRoom> for RenovationCheck {
    async fn process(&self, request: &BookRoom) -> Result<(), BookingError> {
        self.journal
            .borrow_mut()
            .push(format!("Zimmer {} wird renoviert", request.room));
        Err(BookingError::UnderRenovation)
    }
}

/// Schickt nach jeder Buchung eine Bestätigung; der Versand scheitert immer, das schreibt sie ins Protokoll.
pub struct ConfirmationMail {
    pub journal: Journal,
}

impl RequestPostProcessor<BookRoom> for ConfirmationMail {
    async fn process(&self, _request: &BookRoom, booking: &u64) -> Result<(), BookingError> {
        self.journal
            .borrow_mut()
            .push(format!("Bestätigung für Buchung {booking} nicht versendet"));
        Err(BookingError::ConfirmationFailed)
    }
}

/// Die Rezeption: Sie schreibt jeden Fehler einer Buchung, den sie bekommt, ins Protokoll und reicht ihn weiter.
pub struct FrontDesk {
    pub journal: Journal,
}

impl RequestErrorHandler<BookRoom> for FrontDesk {
    async fn handle(&self, request: &BookRoom, error: BookingError) -> Result<u64, BookingError> {
        self.journal.borrow_mut().push(format!(
            "Rezeption meldet für Zimmer {}: {error:?}",
            request.room
        ));
        Err(error)
    }
}

/// Storniert die Buchung mit dieser Nummer.
pub struct CancelBooking {
    pub booking: u64,
}

impl Request for CancelBooking {
    type Response = ();
    type Error = BookingError;
}

/// Ein Hotel ohne Erstattung: Es schreibt jede Stornierung ins Protokoll und lehnt sie ab.
pub struct NoRefunds {
    pub journal: Journal,
}

impl RequestHandler<CancelBooking> for NoRefunds {
    async fn handle(&self, request: CancelBooking) -> Result<(), BookingError> {
        self.journal
            .borrow_mut()
            .push(format!("Storno für Buchung {} abgelehnt", request.booking));
        Err(BookingError::NotRefundable)
    }
}
