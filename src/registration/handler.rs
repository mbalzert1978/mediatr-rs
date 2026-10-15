//! Wie ein Handler in den Mediator kommt.

use std::fmt;
use std::marker::PhantomData;

use mediatr_provider::Registration;

use crate::{ForeachAwaitPublisher, Mediator, WeakMediator};

/// Wie der Mediator einen Handler vom Typ `H` bekommt; [`MediatorBuilder::register`](crate::MediatorBuilder::register), [`MediatorBuilder::register_stream`](crate::MediatorBuilder::register_stream) und [`MediatorBuilder::subscribe`](crate::MediatorBuilder::subscribe) nehmen ihn.
/// `S` und `P` sind Notification-Publisher und Behaviors des Mediators, für den der Handler bestimmt ist.
pub struct Handler<H, S = ForeachAwaitPublisher, P = ()> {
    pub(super) registration: Registration<H>,
    types: PhantomData<(S, P)>,
}

impl<H: 'static, S: 'static, P: 'static> Handler<H, S, P> {
    /// Handler `handler`; jede Nachricht erreicht dasselbe Objekt.
    #[must_use]
    pub fn instance(handler: H) -> Self {
        Self::new(Registration::instance(handler))
    }

    /// Handler aus `make`; jede Nachricht baut ihn damit neu und gibt ihm den [`Mediator`] als [`Sender`](crate::Sender) und [`Publisher`](crate::Publisher).
    #[must_use]
    pub fn transient(make: impl Fn(Mediator<S, P>) -> H + 'static) -> Self {
        Self::new(Registration::transient(move |provider| {
            make(Mediator::new(provider.clone()))
        }))
    }

    /// Handler aus `make`; die erste Nachricht baut ihn damit, jede weitere erreicht denselben Handler.
    ///
    /// Der Handler bekommt einen [`WeakMediator`] als [`Sender`](crate::Sender) und [`Publisher`](crate::Publisher), damit er den Mediator nicht selbst am Leben hält.
    #[must_use]
    pub fn singleton(make: impl Fn(WeakMediator<S, P>) -> H + 'static) -> Self {
        Self::new(Registration::singleton(move |provider| {
            make(WeakMediator {
                provider: provider.downgrade(),
                types: PhantomData,
            })
        }))
    }

    /// Handler, den `registration` baut.
    fn new(registration: Registration<H>) -> Self {
        Self {
            registration,
            types: PhantomData,
        }
    }
}

impl<H, S, P> fmt::Debug for Handler<H, S, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handler")
            .field("registration", &self.registration)
            .finish()
    }
}
