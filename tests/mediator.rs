//! Tests für den Abschnitt „Mediator — Fassade und DI“ in `TESTLIST.md`.

use std::cell::Cell;
use std::rc::Rc;

use mediatr_rs::{Mediator, MissingHandler, Request, RequestHandler, Sender, WeakMediator};

#[derive(Clone)]
struct Customer {
    id: u32,
    name: String,
    vip: bool,
}

/// Die VIP-Kundin mit der Nummer 7, die in den Bestell-Tests bestellt.
fn ada() -> Customer {
    Customer {
        id: 7,
        name: "Ada".to_owned(),
        vip: true,
    }
}

struct GetCustomer {
    id: u32,
}

#[derive(Debug)]
enum CustomerError {
    NotFound,
    MissingHandler,
}

impl From<MissingHandler> for CustomerError {
    fn from(_: MissingHandler) -> Self {
        Self::MissingHandler
    }
}

impl Request for GetCustomer {
    type Response = Customer;
    type Error = CustomerError;
}

/// Kennt alle Kunden; bekommt sie über den Konstruktor.
struct CustomerDirectory {
    customers: Vec<Customer>,
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

struct PlaceOrder {
    customer_id: u32,
    amount: u32,
}

#[derive(Debug, PartialEq)]
struct Confirmation {
    customer: String,
    total: u32,
}

#[derive(Debug)]
enum OrderError {
    /// Der Kunde ließ sich nicht holen; der Grund bleibt erhalten.
    Customer(CustomerError),
    MissingHandler,
}

impl From<MissingHandler> for OrderError {
    fn from(_: MissingHandler) -> Self {
        Self::MissingHandler
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

const VIP_DISCOUNT: u32 = 10;

/// Nimmt Bestellungen an; holt den Kunden über den Sender, den er per Konstruktor bekommt.
struct PlaceOrderHandler<S> {
    sender: S,
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

#[tokio::test]
async fn handler_processes_data_fetched_through_injected_sender() {
    let mediator = Mediator::builder()
        .register(CustomerDirectory {
            customers: vec![ada()],
        })
        .transient(|sender| PlaceOrderHandler { sender })
        .build();

    let confirmation = mediator
        .send(PlaceOrder {
            customer_id: 7,
            amount: 100,
        })
        .await
        .unwrap();

    assert_eq!(
        confirmation,
        Confirmation {
            customer: "Ada".to_owned(),
            total: 90,
        }
    );
}

#[tokio::test]
async fn handler_wraps_failed_inner_request_in_own_error() {
    let mediator = Mediator::builder()
        .register(CustomerDirectory { customers: vec![] })
        .transient(|sender| PlaceOrderHandler { sender })
        .build();

    let result = mediator
        .send(PlaceOrder {
            customer_id: 7,
            amount: 100,
        })
        .await;

    assert!(matches!(
        result,
        Err(OrderError::Customer(CustomerError::NotFound))
    ));
}

#[tokio::test]
async fn transient_builds_handler_on_every_send() {
    let built = Rc::new(Cell::new(0_u32));
    let counter = Rc::clone(&built);
    let mediator = Mediator::builder()
        .register(CustomerDirectory {
            customers: vec![ada()],
        })
        .transient(move |sender| {
            counter.set(counter.get().saturating_add(1));
            PlaceOrderHandler { sender }
        })
        .build();

    for amount in [100, 250] {
        mediator
            .send(PlaceOrder {
                customer_id: 7,
                amount,
            })
            .await
            .unwrap();
    }

    assert_eq!(built.get(), 2);
}

#[tokio::test]
async fn singleton_builds_handler_only_on_first_send() {
    let built = Rc::new(Cell::new(0_u32));
    let counter = Rc::clone(&built);
    let mediator = Mediator::builder()
        .register(CustomerDirectory {
            customers: vec![ada()],
        })
        .singleton(move |sender| {
            counter.set(counter.get().saturating_add(1));
            PlaceOrderHandler { sender }
        })
        .build();

    for amount in [100, 250] {
        mediator
            .send(PlaceOrder {
                customer_id: 7,
                amount,
            })
            .await
            .unwrap();
    }

    assert_eq!(built.get(), 1);
}

/// Nimmt Bestellungen an wie [`PlaceOrderHandler`] und hält dafür die Leitung zum Lager offen; beim Freigeben legt er auf.
struct WarehouseOrderHandler {
    orders: PlaceOrderHandler<WeakMediator>,
    line_open: Rc<Cell<bool>>,
}

impl RequestHandler<PlaceOrder> for WarehouseOrderHandler {
    async fn handle(&self, order: PlaceOrder) -> Result<Confirmation, OrderError> {
        self.orders.handle(order).await
    }
}

impl Drop for WarehouseOrderHandler {
    fn drop(&mut self) {
        self.line_open.set(false);
    }
}

#[tokio::test]
async fn singleton_handler_with_sender_is_released_with_mediator() {
    let line_open = Rc::new(Cell::new(true));
    let line = Rc::clone(&line_open);
    let mediator = Mediator::builder()
        .register(CustomerDirectory {
            customers: vec![ada()],
        })
        .singleton(move |sender| WarehouseOrderHandler {
            orders: PlaceOrderHandler { sender },
            line_open: Rc::clone(&line),
        })
        .build();
    mediator
        .send(PlaceOrder {
            customer_id: 7,
            amount: 100,
        })
        .await
        .unwrap();

    drop(mediator);

    assert!(!line_open.get());
}

/// Attrappe für den Sender: antwortet auf jede Kundenabfrage mit derselben Kundin, ohne Mediator.
struct FixedCustomer(Customer);

impl Sender<GetCustomer> for FixedCustomer {
    async fn send(&self, _request: GetCustomer) -> Result<Customer, CustomerError> {
        Ok(self.0.clone())
    }
}

#[tokio::test]
async fn handler_processes_response_of_fake_sender() {
    let handler = PlaceOrderHandler {
        sender: FixedCustomer(ada()),
    };

    let confirmation = handler
        .handle(PlaceOrder {
            customer_id: 7,
            amount: 100,
        })
        .await
        .unwrap();

    assert_eq!(
        confirmation,
        Confirmation {
            customer: "Ada".to_owned(),
            total: 90,
        }
    );
}
