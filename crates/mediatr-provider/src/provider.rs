//! Der Provider liefert Dienste, die sein Builder registriert hat.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;

/// Dienste verschiedener Typen, je Typ einer.
// Rc reicht, solange kein Testlisten-Punkt Threads verlangt; dann Arc.
type Services = HashMap<TypeId, Rc<dyn Any>>;

/// Liefert Dienste nach ihrem Typ.
///
/// Entsteht über [`Provider::builder`].
#[derive(Debug, Clone)]
pub struct Provider {
    services: Services,
}

/// Sammelt Dienste und baut daraus einen [`Provider`].
#[derive(Debug, Clone, Default)]
pub struct ProviderBuilder {
    services: Services,
}

impl Provider {
    /// Beginnt einen Provider ohne Dienste.
    #[must_use]
    pub fn builder() -> ProviderBuilder {
        ProviderBuilder::default()
    }

    /// Liefert den Dienst vom Typ `T`.
    #[must_use]
    pub fn get<T: 'static>(&self) -> Option<Rc<T>> {
        Rc::clone(self.services.get(&TypeId::of::<T>())?)
            .downcast()
            .ok()
    }
}

impl ProviderBuilder {
    /// Registriert `service`; jede Abfrage liefert dasselbe Objekt.
    #[must_use]
    pub fn instance<T: 'static>(mut self, service: T) -> Self {
        self.services.insert(TypeId::of::<T>(), Rc::new(service));
        self
    }

    /// Baut den Provider mit allen registrierten Diensten.
    #[must_use]
    pub fn build(self) -> Provider {
        Provider {
            services: self.services,
        }
    }
}
