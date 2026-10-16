//! Benchmarks für `send`, `publish` und `create_stream` eines Preisdienstes, auf einem und auf mehreren Threads.

use std::hint::black_box;
use std::pin::pin;
use std::sync::Barrier;
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};

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

/// So viele Threads schicken im Bench mit mehreren Threads gleichzeitig.
const THREADS: usize = 4;

/// Schickt `rounds` Preisabfragen über `mediator`, sobald alle Threads an `start` angekommen sind, und misst die Zeit dafür.
fn ask_prices(mediator: &Mediator, rounds: u64, start: &Barrier) -> Duration {
    start.wait();
    let began = Instant::now();
    for _ in 0..rounds {
        let _price = black_box(run(mediator.send(GetPrice { item: black_box(7) })));
    }
    began.elapsed()
}

/// Lässt [`THREADS`] Threads gleichzeitig je `rounds` Preisabfragen über ihren Klon von `mediator` schicken; liefert die Zeit des langsamsten.
fn ask_prices_on_threads(mediator: &Mediator, rounds: u64) -> Duration {
    let start = &Barrier::new(THREADS);
    thread::scope(|scope| {
        let clerks: Vec<_> = (0..THREADS)
            .map(|_| {
                let mediator = mediator.clone();
                scope.spawn(move || ask_prices(&mediator, rounds, start))
            })
            .collect();
        clerks
            .into_iter()
            .filter_map(|clerk| clerk.join().ok())
            .max()
            .unwrap_or_default()
    })
}

/// Misst eine Runde, in der jeder von [`THREADS`] Threads gleichzeitig eine Preisabfrage schickt.
// Grund: Jedes `get` im Provider nimmt einen Lock; ein Bench auf einem Thread sieht nicht, ob die Threads dort aufeinander warten.
fn send_on_threads(c: &mut Criterion) {
    let mediator = Mediator::builder()
        .register(Handler::instance(PriceList))
        .build();
    c.bench_function("send auf 4 Threads gleichzeitig", |b| {
        b.iter_custom(|rounds| ask_prices_on_threads(&mediator, rounds));
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

criterion_group!(benches, send, send_on_threads, publish, create_stream);
criterion_main!(benches);
