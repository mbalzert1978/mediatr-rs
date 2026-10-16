use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use futures_core::Stream;
use mediatr_rs::{
    Handler, Mediator, Next, Notification, NotificationHandler, PipelineBehavior, Request,
    RequestHandler, SendError, Sender, StreamRequest, StreamRequestHandler,
};

use crate::common::Journal;

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

/// Warenkorb wie [`Cart`], der dafür die Kasse offen hält; beim Freigeben schließt er sie.
pub struct CheckoutCart {
    pub cart: Cart,
    pub till_open: Arc<AtomicBool>,
}

impl RequestHandler<AddToCart> for CheckoutCart {
    async fn handle(&self, request: AddToCart) -> Result<u32, SendError> {
        self.cart.handle(request).await
    }
}

impl Drop for CheckoutCart {
    fn drop(&mut self) {
        self.till_open.store(false, Ordering::SeqCst);
    }
}

/// Mediator, der nur den Warenkorb kennt, als `Handler::scoped`.
pub fn cart_mediator() -> Mediator {
    Mediator::builder()
        .register(Handler::scoped(|_| Cart::default()))
        .build()
}

/// Zählt, wie oft ein Artikel in den Warenkorb soll, und reicht ihn an den Warenkorb weiter.
pub struct CartAudit {
    pub seen: Arc<AtomicU32>,
}

impl PipelineBehavior<AddToCart> for CartAudit {
    async fn handle(&self, request: AddToCart, next: Next<AddToCart>) -> Result<u32, SendError> {
        self.seen.fetch_add(1, Ordering::SeqCst);
        next.run(request).await
    }
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
pub struct GiftWrapHandler<S> {
    pub sender: S,
}

impl<S: Sender<AddToCart>> RequestHandler<AddGiftWrap> for GiftWrapHandler<S> {
    async fn handle(&self, _request: AddGiftWrap) -> Result<u32, SendError> {
        self.sender
            .send(AddToCart {
                price: GIFT_WRAP_PRICE,
            })
            .await
    }
}

/// Der Kunde hat den Warenkorb bezahlt; `total` ist die Summe, in Rappen.
pub struct CartCheckedOut {
    pub total: u32,
}

impl Notification for CartCheckedOut {
    type Error = SendError;
}

/// Druckt zu jedem bezahlten Warenkorb einen Beleg ins Protokoll.
pub struct ReceiptPrinter {
    pub journal: Journal,
}

impl NotificationHandler<CartCheckedOut> for ReceiptPrinter {
    async fn handle(&self, event: &CartCheckedOut) -> Result<(), SendError> {
        self.journal.push(format!("Beleg über {}", event.total));
        Ok(())
    }
}

/// Listet die Preise der Artikel im Warenkorb auf, einen nach dem anderen, in Rappen.
pub struct ListCartItems;

impl StreamRequest for ListCartItems {
    type Item = u32;
    type Error = SendError;
}

/// Der Inhalt eines Warenkorbs: die Preise seiner Artikel in der Reihenfolge, in der sie hineinkamen.
pub struct CartContents {
    pub prices: Vec<u32>,
}

impl StreamRequestHandler<ListCartItems> for CartContents {
    fn handle(
        &self,
        _request: ListCartItems,
    ) -> impl Stream<Item = Result<u32, SendError>> + use<> {
        futures_util::stream::iter(self.prices.clone().into_iter().map(Ok))
    }
}

/// Die Arbeitseinheit einer Web-Anfrage; sammelt die Bestellzeilen, bis die Anfrage sie speichert.
#[derive(Default)]
pub struct UnitOfWork {
    pending_lines: AtomicU32,
}

/// Fügt der Bestellung eine Zeile hinzu.
pub struct AddOrderLine;

impl Request for AddOrderLine {
    type Response = ();
    type Error = SendError;
}

/// Merkt sich jede neue Bestellzeile in der Arbeitseinheit, die er per Konstruktor bekommt.
pub struct OrderLineRecorder {
    pub unit_of_work: Arc<UnitOfWork>,
}

impl RequestHandler<AddOrderLine> for OrderLineRecorder {
    async fn handle(&self, _request: AddOrderLine) -> Result<(), SendError> {
        self.unit_of_work
            .pending_lines
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// [`OrderLineRecorder`] als `Handler::scoped_from`; seine Bauanleitung holt die [`UnitOfWork`] aus dem Scope.
pub fn order_line_recorder() -> Handler<OrderLineRecorder> {
    Handler::scoped_from(|_, provider| {
        Ok(OrderLineRecorder {
            unit_of_work: provider.get()?,
        })
    })
}

/// Fragt, wie viele Bestellzeilen noch nicht gespeichert sind.
pub struct CountOrderLines;

impl Request for CountOrderLines {
    type Response = u32;
    type Error = SendError;
}

/// Zählt die Bestellzeilen in der Arbeitseinheit, die er per Konstruktor bekommt.
pub struct OrderLineCounter {
    pub unit_of_work: Arc<UnitOfWork>,
}

impl RequestHandler<CountOrderLines> for OrderLineCounter {
    async fn handle(&self, _request: CountOrderLines) -> Result<u32, SendError> {
        Ok(self.unit_of_work.pending_lines.load(Ordering::SeqCst))
    }
}

/// Speichert die offenen Bestellzeilen der Arbeitseinheit, die er per Konstruktor bekommt, sobald der Kunde bezahlt hat.
pub struct OrderLineCommitter {
    pub unit_of_work: Arc<UnitOfWork>,
}

impl NotificationHandler<CartCheckedOut> for OrderLineCommitter {
    async fn handle(&self, _event: &CartCheckedOut) -> Result<(), SendError> {
        self.unit_of_work.pending_lines.store(0, Ordering::SeqCst);
        Ok(())
    }
}

/// Listet die offenen Bestellzeilen auf, eine Zeilennummer nach der anderen, ab 1.
pub struct ListOrderLines;

impl StreamRequest for ListOrderLines {
    type Item = u32;
    type Error = SendError;
}

/// Nummeriert die offenen Bestellzeilen der Arbeitseinheit, die er per Konstruktor bekommt.
pub struct OrderLineLister {
    pub unit_of_work: Arc<UnitOfWork>,
}

impl StreamRequestHandler<ListOrderLines> for OrderLineLister {
    fn handle(
        &self,
        _request: ListOrderLines,
    ) -> impl Stream<Item = Result<u32, SendError>> + use<> {
        let pending = self.unit_of_work.pending_lines.load(Ordering::SeqCst);
        futures_util::stream::iter((1..=pending).map(Ok))
    }
}
