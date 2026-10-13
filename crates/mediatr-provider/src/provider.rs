//! Der Provider liefert Dienste, die sein Builder registriert hat.

use std::any::{Any, TypeId};
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// Bauanleitungen verschiedener Typen, je Typ eine; jede ist ein [`Recipe`] ihres Typs.
// Rc reicht, solange kein Testlisten-Punkt Threads verlangt; dann Arc.
type Services = HashMap<TypeId, Rc<dyn Any>>;

/// Dienste, die ein Scope schon gebaut hat, je Typ einer.
type Built = RefCell<HashMap<TypeId, Rc<dyn Any>>>;

/// Liefert bei jedem Aufruf einen Dienst vom Typ `T`, gebaut oder gemerkt im übergebenen Scope.
// dyn: `get` kennt nur `T`, nicht den Typ der Bauanleitung. Bauanleitungen verschiedener
// Typen (aus `instance`, `transient`, `singleton`, `scoped`) liegen deshalb hinter einem gemeinsamen Typ je `T`.
type Recipe<T> = Box<dyn Fn(&Built) -> Rc<T>>;

/// Liefert Dienste nach ihrem Typ.
///
/// Entsteht über [`Provider::builder`]. Der Provider ist selbst ein Scope.
#[derive(Debug, Clone)]
pub struct Provider {
    services: Services,
    built: Built,
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
        Some(recipe(&self.built))
    }

    /// Beginnt einen neuen Scope mit denselben Diensten; `scoped`-Dienste baut er neu.
    #[must_use]
    pub fn scope(&self) -> Self {
        Self {
            services: self.services.clone(),
            built: Built::default(),
        }
    }
}

impl ProviderBuilder {
    /// Registriert `service`; jede Abfrage liefert dasselbe Objekt.
    #[must_use]
    pub fn instance<T: 'static>(self, service: T) -> Self {
        let service = Rc::new(service);
        self.recipe(move |_| Rc::clone(&service))
    }

    /// Registriert `recipe`; jede Abfrage baut den Dienst damit neu.
    #[must_use]
    pub fn transient<T: 'static>(self, recipe: impl Fn() -> T + 'static) -> Self {
        self.recipe(move |_| Rc::new(recipe()))
    }

    /// Registriert `recipe`; die erste Abfrage baut den Dienst, jede weitere liefert dasselbe Objekt.
    #[must_use]
    pub fn singleton<T: 'static>(self, recipe: impl Fn() -> T + 'static) -> Self {
        let service = OnceCell::new();
        self.recipe(move |_| Rc::clone(service.get_or_init(|| Rc::new(recipe()))))
    }

    /// Registriert `recipe`; die erste Abfrage in einem Scope baut den Dienst, jede weitere im selben Scope liefert dasselbe Objekt.
    #[must_use]
    pub fn scoped<T: 'static>(self, recipe: impl Fn() -> T + 'static) -> Self {
        self.recipe(move |built| scoped_service(built, &recipe))
    }

    /// Baut den Provider mit allen registrierten Diensten.
    #[must_use]
    pub fn build(self) -> Provider {
        Provider {
            services: self.services,
            built: Built::default(),
        }
    }

    fn recipe<T: 'static>(mut self, recipe: impl Fn(&Built) -> Rc<T> + 'static) -> Self {
        let recipe: Recipe<T> = Box::new(recipe);
        self.services.insert(TypeId::of::<T>(), Rc::new(recipe));
        self
    }
}

/// Liefert den Dienst vom Typ `T` aus `built`; fehlt er dort, baut `recipe` ihn, und `built` merkt ihn sich.
fn scoped_service<T: 'static>(built: &Built, recipe: impl Fn() -> T) -> Rc<T> {
    let id = TypeId::of::<T>();
    let cached = built
        .borrow()
        .get(&id)
        .and_then(|service| Rc::clone(service).downcast().ok());
    cached.unwrap_or_else(|| {
        let service = Rc::new(recipe());
        built
            .borrow_mut()
            .insert(id, Rc::clone(&service) as Rc<dyn Any>);
        service
    })
}
