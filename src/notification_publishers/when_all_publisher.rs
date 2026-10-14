//! Die Strategie, die alle Handler gleichzeitig auf einem Thread laufen lässt.

use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use crate::NotificationPublisher;

/// Startet alle Handler gleichzeitig auf demselben Thread: Wartet einer, läuft der nächste weiter.
/// Jeder Handler läuft zu Ende, auch nach einem Fehler; danach liefert `publish` den Fehler des zuerst registrierten Handlers, der gescheitert ist.
///
/// # Achtung
///
/// Hält ein Handler eine `RefCell`-Leihe über ein `await`, kann ein anderer Handler dieselbe `RefCell` leihen wollen;
/// das löst eine Panic aus. Clippy meldet solche Stellen mit `await_holding_refcell_ref`; diesen Lint nicht abschalten.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct WhenAllPublisher;

impl NotificationPublisher for WhenAllPublisher {
    async fn publish<E, F>(calls: Vec<F>) -> Result<(), E>
    where
        F: Future<Output = Result<(), E>>,
    {
        let mut calls: Vec<Call<F>> = calls
            .into_iter()
            .map(|call| Call::Running(Box::pin(call)))
            .collect();
        poll_fn(|cx| poll_all(&mut calls, cx)).await;
        calls.into_iter().filter_map(Call::into_output).collect()
    }
}

/// Treibt jeden laufenden Aufruf aus `calls` weiter; `Poll::Ready`, sobald alle ihr Ergebnis haben.
fn poll_all<F: Future>(calls: &mut [Call<F>], cx: &mut Context<'_>) -> Poll<()> {
    let running = calls
        .iter_mut()
        .map(|call| call.poll(cx))
        .filter(Poll::is_pending)
        .count();
    if running == 0 {
        Poll::Ready(())
    } else {
        Poll::Pending
    }
}

/// Ein Aufruf aus `calls` für [`WhenAllPublisher`], der noch läuft oder schon sein Ergebnis hat.
enum Call<F: Future> {
    // Box::pin, weil `F` nicht `Unpin` sein muss.
    Running(Pin<Box<F>>),
    Done(F::Output),
}

impl<F: Future> Call<F> {
    /// Treibt den Aufruf weiter, solange er läuft; `Poll::Ready`, sobald er sein Ergebnis hat.
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let Self::Running(future) = self else {
            return Poll::Ready(());
        };
        *self = Self::Done(ready!(future.as_mut().poll(cx)));
        Poll::Ready(())
    }

    /// Das Ergebnis des Aufrufs; `None`, solange er läuft.
    fn into_output(self) -> Option<F::Output> {
        match self {
            Self::Done(output) => Some(output),
            Self::Running(_) => None,
        }
    }
}
