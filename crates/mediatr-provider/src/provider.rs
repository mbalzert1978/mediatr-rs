//! Der Provider liefert Dienste, die sein Builder registriert hat.

use std::any::{Any, TypeId};
use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Bauanleitungen verschiedener Typen, je Typ eine; jede ist ein [`Recipe`] ihres Typs.
// Rc reicht, solange kein Testlisten-Punkt Threads verlangt; dann Arc.
type Services = HashMap<TypeId, Rc<dyn Any>>;

/// Liefert bei jedem Aufruf einen Dienst vom Typ `T`.
// dyn: `get` kennt nur `T`, nicht den Typ der Bauanleitung. Bauanleitungen verschiedener
// Typen (aus `instance`, `transient`, `singleton`) liegen deshalb hinter einem gemeinsamen Typ je `T`.
type Recipe<T> = Box<dyn Fn() -> Rc<T>>;

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
        let recipe = self
            .services
            .get(&TypeId::of::<T>())?
            .downcast_ref::<Recipe<T>>()?;
        Some(recipe())
    }
}

impl ProviderBuilder {
    /// Registriert `service`; jede Abfrage liefert dasselbe Objekt.
    #[must_use]
    pub fn instance<T: 'static>(self, service: T) -> Self {
        let service = Rc::new(service);
        self.recipe(move || Rc::clone(&service))
    }

    /// Registriert `recipe`; jede Abfrage baut den Dienst damit neu.
    #[must_use]
    pub fn transient<T: 'static>(self, recipe: impl Fn() -> T + 'static) -> Self {
        self.recipe(move || Rc::new(recipe()))
    }

    /// Registriert `recipe`; die erste Abfrage baut den Dienst, jede weitere liefert dasselbe Objekt.
    #[must_use]
    pub fn singleton<T: 'static>(self, recipe: impl Fn() -> T + 'static) -> Self {
        let service = OnceCell::new();
        self.recipe(move || Rc::clone(service.get_or_init(|| Rc::new(recipe()))))
    }

    /// Baut den Provider mit allen registrierten Diensten.
    #[must_use]
    pub fn build(self) -> Provider {
        Provider {
            services: self.services,
        }
    }

    fn recipe<T: 'static>(mut self, recipe: impl Fn() -> Rc<T> + 'static) -> Self {
        let recipe: Recipe<T> = Box::new(recipe);
        self.services.insert(TypeId::of::<T>(), Rc::new(recipe));
        self
    }
}
