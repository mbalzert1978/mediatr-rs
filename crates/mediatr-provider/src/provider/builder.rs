//! Der Builder, aus dem ein Provider entsteht.

use std::any::TypeId;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use super::{Built, Services, Shared};
use crate::{Provider, Registration};

/// Sammelt Dienste und baut daraus einen [`Provider`].
///
/// Eine Bauanleitung bekommt den abfragenden Provider und holt darüber andere Dienste.
#[derive(Debug, Clone, Default)]
pub struct ProviderBuilder {
    services: Services,
    /// Paare aus Diensttyp und Schlüsseltyp, die [`ProviderBuilder::register`] als einmalig schon aufgenommen hat.
    keys: HashSet<(TypeId, TypeId)>,
}

impl ProviderBuilder {
    /// Nimmt `registration` auf; ist sie einmalig und für ihren Typ schon eine mit demselben Schlüsseltyp aufgenommen, nimmt es sie nicht auf.
    #[must_use]
    pub fn register<T: Send + Sync + 'static>(mut self, registration: Registration<T>) -> Self {
        let id = TypeId::of::<T>();
        if let Some(key) = registration.key
            && !self.keys.insert((id, key))
        {
            return self;
        }
        self.services
            .entry(id)
            .or_default()
            .push(Arc::new(registration.recipe));
        self
    }

    /// Baut den Provider mit allen registrierten Diensten.
    #[must_use]
    pub fn build(self) -> Provider {
        let root = Arc::<Built>::default();
        Provider {
            built: Arc::clone(&root),
            shared: Arc::new(Shared {
                services: self.services,
                root,
                builds: Mutex::default(),
            }),
        }
    }
}
