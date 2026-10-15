use std::cell::RefCell;
use std::rc::Rc;

use mediatr_rs::{Request, RequestErrorAction, RequestHandler, SendError};

/// Bucht das Hotelzimmer mit dieser Nummer und liefert die Buchungsnummer.
#[derive(Clone)]
pub struct BookRoom {
    pub room: u32,
}

/// Warum eine Buchung scheitert.
#[derive(Debug, PartialEq)]
pub enum BookingError {
    FullyBooked,
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
