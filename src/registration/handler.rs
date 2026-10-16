//! Wie ein Handler in den Mediator kommt.

use std::fmt;
use std::marker::PhantomData;

use mediatr_provider::Registration;

use crate::{ForeachAwaitPublisher, Mediator, WeakMediator};

/// Wie der Mediator einen Handler vom Typ `H` bekommt; [`MediatorBuilder::register`](crate::MediatorBuilder::register), [`MediatorBuilder::register_stream`](crate::MediatorBuilder::register_stream) und [`MediatorBuilder::subscribe`](crate::MediatorBuilder::subscribe) nehmen ihn.
/// `S` ist der Notification-Publisher des Mediators, für den der Handler bestimmt ist.
pub struct Handler<H, S = ForeachAwaitPublisher> {
    pub(super) registration: Registration<H>,
    publisher: PhantomData<S>,
}

impl<H: Send + Sync + 'static, S: 'static> Handler<H, S> {
    /// Handler `handler`; jede Nachricht erreicht dasselbe Objekt.
    #[must_use]
    pub fn instance(handler: H) -> Self {
        Self::new(Registration::instance(handler))
    }

    /// Handler aus `make`; jede Nachricht baut ihn damit neu und gibt ihm den [`Mediator`] als [`Sender`](crate::Sender) und [`Publisher`](crate::Publisher).
    #[must_use]
    pub fn transient(make: impl Fn(Mediator<S>) -> H + Send + Sync + 'static) -> Self {
        Self::new(Registration::transient(move |provider| {
            Ok(make(Mediator::new(provider.clone())))
        }))
    }

    /// Handler aus `make`; die erste Nachricht baut ihn damit, jede weitere erreicht denselben Handler.
    ///
    /// Der Handler bekommt einen [`WeakMediator`] als [`Sender`](crate::Sender) und [`Publisher`](crate::Publisher), damit er den Mediator nicht selbst am Leben hält.
    #[must_use]
    pub fn singleton(make: impl Fn(WeakMediator<S>) -> H + Send + Sync + 'static) -> Self {
        Self::new(Registration::singleton(move |provider| {
            Ok(make(WeakMediator {
                provider: provider.downgrade(),
                publisher: PhantomData,
            }))
        }))
    }

    /// Handler aus `make`; die erste Nachricht in einem Scope aus [`Mediator::scope`] baut ihn, jede weitere im selben Scope erreicht denselben Handler.
    ///
    /// Der Handler bekommt den [`WeakMediator`] seines Scopes als [`Sender`](crate::Sender) und [`Publisher`](crate::Publisher), damit er den Scope nicht selbst am Leben hält.
    /// Schicken zwei Aufrufe im selben Scope gleichzeitig, kann `make` zweimal laufen; beide bekommen denselben Handler.
    #[must_use]
    pub fn scoped(make: impl Fn(WeakMediator<S>) -> H + Send + Sync + 'static) -> Self {
        Self::new(Registration::scoped(move |provider| {
            Ok(make(WeakMediator {
                provider: provider.downgrade(),
                publisher: PhantomData,
            }))
        }))
    }

    /// Handler, den `registration` baut.
    fn new(registration: Registration<H>) -> Self {
        Self {
            registration,
            publisher: PhantomData,
        }
    }
}

impl<H, S> fmt::Debug for Handler<H, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handler")
            .field("registration", &self.registration)
            .finish()
    }
}
