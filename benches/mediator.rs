//! Benchmarks für `send`, `publish` und `create_stream` eines Preisdienstes.

use std::hint::black_box;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use criterion::{Criterion, criterion_group, criterion_main};
use futures_core::Stream;
use futures_util::StreamExt;
use mediatr_rs::{
    Handler, Mediator, Next, Notification, NotificationHandler, PipelineBehavior, Publisher,
    Request, RequestHandler, SendError, Sender, StreamRequest, StreamRequestHandler, StreamSender,
};

/// Fragt den Preis eines Artikels in Rappen ab.
struct GetPrice {
    item: u32,
}

impl Request for GetPrice {
    type Response = u32;
    type Error = SendError;
}

/// Die Preisliste: Jeder Artikel kostet seine Nummer mal 100 Rappen.
struct PriceList;

impl RequestHandler<GetPrice> for PriceList {
    async fn handle(&self, request: GetPrice) -> Result<u32, SendError> {
        Ok(request.item.saturating_mul(100))
    }
}

/// Eine Prüfschicht, die jede Preisabfrage nur weiterreicht; `N` macht drei Schichten zu drei Typen.
struct Checkpoint<const N: usize>;

impl<const N: usize> PipelineBehavior<GetPrice> for Checkpoint<N> {
    async fn handle(&self, request: GetPrice, next: Next<GetPrice>) -> Result<u32, SendError> {
        next.run(request).await
    }
}

/// Meldet, dass sich der Preis eines Artikels geändert hat.
struct PriceChanged {
    item: u32,
}

impl Notification for PriceChanged {
    type Error = SendError;
}

/// Eine Filiale, die Preisänderungen annimmt; `N` macht drei Filialen zu drei Typen.
struct Branch<const N: usize>;

impl<const N: usize> NotificationHandler<PriceChanged> for Branch<N> {
    async fn handle(&self, notification: &PriceChanged) -> Result<(), SendError> {
        black_box(notification.item);
        Ok(())
    }
}

/// Fragt die Preise der ersten `count` Artikel ab.
struct ListPrices {
    count: u32,
}

impl StreamRequest for ListPrices {
    type Item = u32;
    type Error = SendError;
}

impl StreamRequestHandler<ListPrices> for PriceList {
    fn handle(&self, request: ListPrices) -> impl Stream<Item = Result<u32, SendError>> + use<> {
        futures_util::stream::iter((0..request.count).map(|item| Ok(item.saturating_mul(100))))
    }
}

/// Führt `future` bis zum Ende aus.
// ponytail: dreht sich, solange `future` wartet; alle Handler hier sind sofort fertig, darum misst das nur den Mediator. Für echtes Warten eine Runtime nehmen.
fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
    }
}

fn send(c: &mut Criterion) {
    let bare = Mediator::builder()
        .register(Handler::instance(PriceList))
        .build();
    c.bench_function("send ohne Behaviors", |b| {
        b.iter(|| run(bare.send(GetPrice { item: black_box(7) })));
    });

    let layered = Mediator::builder()
        .open_behavior(Checkpoint::<1>)
        .open_behavior(Checkpoint::<2>)
        .open_behavior(Checkpoint::<3>)
        .register(Handler::instance(PriceList))
        .build();
    c.bench_function("send mit 3 Behaviors", |b| {
        b.iter(|| run(layered.send(GetPrice { item: black_box(7) })));
    });
}

fn publish(c: &mut Criterion) {
    let mediator = Mediator::builder()
        .subscribe(Handler::instance(Branch::<1>))
        .subscribe(Handler::instance(Branch::<2>))
        .subscribe(Handler::instance(Branch::<3>))
        .build();
    c.bench_function("publish an 3 Handler", |b| {
        b.iter(|| run(mediator.publish(PriceChanged { item: black_box(7) })));
    });
}

fn create_stream(c: &mut Criterion) {
    let mediator = Mediator::builder()
        .register_stream(Handler::instance(PriceList))
        .build();
    c.bench_function("create_stream mit 10 Elementen", |b| {
        b.iter(|| {
            run(mediator
                .create_stream(ListPrices {
                    count: black_box(10),
                })
                .collect::<Vec<_>>())
        });
    });
}

criterion_group!(benches, send, publish, create_stream);
criterion_main!(benches);
