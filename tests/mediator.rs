//! Tests für den Abschnitt „Mediator — Fassade und DI“ in `TESTLIST.md`.

use std::cell::Cell;
use std::rc::Rc;

use mediatr_rs::{Mediator, MissingHandler, Request, RequestHandler, Sender};

#[derive(Clone)]
struct Customer {
    id: u32,
    name: String,
    vip: bool,
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
struct PlaceOrderHandler {
    sender: Sender,
}

impl RequestHandler<PlaceOrder> for PlaceOrderHandler {
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
            customers: vec![Customer {
                id: 7,
                name: "Ada".to_owned(),
                vip: true,
            }],
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
            customers: vec![Customer {
                id: 7,
                name: "Ada".to_owned(),
                vip: false,
            }],
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
