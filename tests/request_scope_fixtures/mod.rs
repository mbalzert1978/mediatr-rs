use std::sync::atomic::{AtomicU32, Ordering};

use mediatr_rs::{Request, RequestHandler, SendError};

/// Legt einen Artikel in den Warenkorb; die Antwort ist die Summe aller Artikel bisher, in Rappen.
pub struct AddToCart {
    pub price: u32,
}

impl Request for AddToCart {
    type Response = u32;
    type Error = SendError;
}

/// Der Warenkorb einer Web-Anfrage; zählt die Preise der Artikel zusammen.
#[derive(Default)]
pub struct Cart {
    total: AtomicU32,
}

impl RequestHandler<AddToCart> for Cart {
    async fn handle(&self, request: AddToCart) -> Result<u32, SendError> {
        let before = self.total.fetch_add(request.price, Ordering::SeqCst);
        Ok(before.saturating_add(request.price))
    }
}
