use std::sync::atomic::{AtomicU32, Ordering};

use mediatr_rs::{Handler, Mediator, Request, RequestHandler, SendError, Sender};

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

/// Mediator, der nur den Warenkorb kennt, als `Handler::scoped`.
pub fn cart_mediator() -> Mediator {
    Mediator::builder()
        .register(Handler::scoped(|_| Cart::default()))
        .build()
}

/// Preis der Geschenkverpackung, in Rappen.
pub const GIFT_WRAP_PRICE: u32 = 50;

/// Packt die Bestellung als Geschenk ein; die Antwort ist die Summe des Warenkorbs mit der Verpackung.
pub struct AddGiftWrap;

impl Request for AddGiftWrap {
    type Response = u32;
    type Error = SendError;
}

/// Legt die Geschenkverpackung über den Sender, den er per Konstruktor bekommt, in den Warenkorb.
pub struct GiftWrapHandler {
    pub sender: Mediator,
}

impl RequestHandler<AddGiftWrap> for GiftWrapHandler {
    async fn handle(&self, _request: AddGiftWrap) -> Result<u32, SendError> {
        self.sender
            .send(AddToCart {
                price: GIFT_WRAP_PRICE,
            })
            .await
    }
}
