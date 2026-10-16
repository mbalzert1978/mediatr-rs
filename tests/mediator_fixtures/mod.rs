use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use futures_core::Stream;
use mediatr_rs::{
    Mediator, Next, Notification, NotificationHandler, PipelineBehavior, Request, RequestHandler,
    SendError, Sender, StreamNext, StreamPipelineBehavior, StreamRequest, StreamRequestHandler,
    WeakMediator,
};

#[derive(Clone)]
pub struct Customer {
    pub id: u32,
    pub name: String,
    pub vip: bool,
}

/// Die VIP-Kundin mit der Nummer 7, die in den Bestell-Tests bestellt.
pub fn ada() -> Customer {
    Customer {
        id: 7,
        name: "Ada".to_owned(),
        vip: true,
    }
}

pub struct GetCustomer {
    pub id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CustomerError {
    NotFound,
    Undeliverable(SendError),
}

impl From<SendError> for CustomerError {
    fn from(error: SendError) -> Self {
        Self::Undeliverable(error)
    }
}

impl Request for GetCustomer {
    type Response = Customer;
    type Error = CustomerError;
}

/// Kennt alle Kunden; bekommt sie über den Konstruktor.
pub struct CustomerDirectory {
    pub customers: Vec<Customer>,
}

impl RequestHandler<GetCustomer> for CustomerDirectory {
    async fn handle(&self, request: GetCustomer) -> Result<Customer, CustomerError> {
        self.customers
            .iter()
            .find(|customer| customer.id == request.id)
            .cloned()
            .ok_or(CustomerError::NotFound)
    }
}

/// Fragt alle Kunden ab; der Handler liefert ihre Namen als Stream.
pub struct ListCustomers;

impl StreamRequest for ListCustomers {
    type Item = String;
    type Error = CustomerError;
}

impl StreamRequestHandler<ListCustomers> for CustomerDirectory {
    fn handle(
        &self,
        _request: ListCustomers,
    ) -> impl Stream<Item = Result<String, CustomerError>> + use<> {
        let names: Vec<_> = self
            .customers
            .iter()
            .map(|customer| Ok(customer.name.clone()))
            .collect();
        futures_util::stream::iter(names)
    }
}

/// Meldet, dass ein neuer Kunde dazugekommen ist.
pub struct CustomerJoined;

impl Notification for CustomerJoined {
    type Error = CustomerError;
}

/// Schickt jedem neuen Kunden eine Begrüßung und zählt die verschickten.
pub struct WelcomeMail {
    pub sent: Arc<AtomicU32>,
}

impl NotificationHandler<CustomerJoined> for WelcomeMail {
    async fn handle(&self, _event: &CustomerJoined) -> Result<(), CustomerError> {
        self.sent.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// Stellt jedem neuen Kunden eine Kundenkarte aus und zählt die ausgestellten.
pub struct LoyaltyCard {
    pub issued: Arc<AtomicU32>,
}

impl NotificationHandler<CustomerJoined> for LoyaltyCard {
    async fn handle(&self, _event: &CustomerJoined) -> Result<(), CustomerError> {
        self.issued.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

pub struct PlaceOrder {
    pub customer_id: u32,
    pub amount: u32,
}

#[derive(Debug, PartialEq)]
pub struct Confirmation {
    pub customer: String,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OrderError {
    /// Der Kunde ließ sich nicht holen; der Grund bleibt erhalten.
    Customer(CustomerError),
    Undeliverable,
}

impl From<SendError> for OrderError {
    fn from(_: SendError) -> Self {
        Self::Undeliverable
    }
}

impl From<CustomerError> for OrderError {
    fn from(error: CustomerError) -> Self {
        Self::Customer(error)
    }
}

impl Request for PlaceOrder {
    type Response = Confirmation;
    type Error = OrderError;
}

pub const VIP_DISCOUNT: u32 = 10;

/// Nimmt Bestellungen an; holt den Kunden über den Sender, den er per Konstruktor bekommt.
pub struct PlaceOrderHandler<S> {
    pub sender: S,
}

impl<S: Sender<GetCustomer>> RequestHandler<PlaceOrder> for PlaceOrderHandler<S> {
    async fn handle(&self, order: PlaceOrder) -> Result<Confirmation, OrderError> {
        let customer = self
            .sender
            .send(GetCustomer {
                id: order.customer_id,
            })
            .await?;
        let discount = if customer.vip { VIP_DISCOUNT } else { 0 };
        Ok(Confirmation {
            customer: customer.name,
            total: order.amount.saturating_sub(discount),
        })
    }
}

/// Nimmt Bestellungen an wie [`PlaceOrderHandler`] und hält dafür die Leitung zum Lager offen; beim Freigeben legt er auf.
pub struct WarehouseOrderHandler {
    pub orders: PlaceOrderHandler<WeakMediator>,
    pub line_open: Arc<AtomicBool>,
}

impl RequestHandler<PlaceOrder> for WarehouseOrderHandler {
    async fn handle(&self, order: PlaceOrder) -> Result<Confirmation, OrderError> {
        self.orders.handle(order).await
    }
}

impl Drop for WarehouseOrderHandler {
    fn drop(&mut self) {
        self.line_open.store(false, Ordering::SeqCst);
    }
}

/// Attrappe für den Sender: antwortet auf jede Kundenabfrage mit derselben Kundin, ohne Mediator.
pub struct FixedCustomer(pub Customer);

impl Sender<GetCustomer> for FixedCustomer {
    async fn send(&self, _request: GetCustomer) -> Result<Customer, CustomerError> {
        Ok(self.0.clone())
    }
}

/// Zählt jeden Request und jeden Stream-Request, der durch die Pipeline geht, und lässt ihn dann weiter.
pub struct Turnstile {
    pub passed: Arc<AtomicU32>,
}

impl<R: Request> PipelineBehavior<R> for Turnstile {
    async fn handle(&self, request: R, next: Next<R>) -> Result<R::Response, R::Error> {
        self.passed.fetch_add(1, Ordering::SeqCst);
        next.run(request).await
    }
}

impl<R: StreamRequest> StreamPipelineBehavior<R> for Turnstile {
    fn handle(
        &self,
        request: R,
        next: StreamNext<R>,
    ) -> impl Stream<Item = Result<R::Item, R::Error>> + use<R> {
        self.passed.fetch_add(1, Ordering::SeqCst);
        next.run(request)
    }
}

/// Ein Laden, der seinen Mediator als Feld hält, ohne dessen Behaviors zu nennen.
pub struct Shop {
    pub mediator: Mediator,
}
