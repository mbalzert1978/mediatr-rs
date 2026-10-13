//! Der Provider liefert Dienste, die sein Builder registriert hat.

use std::any::{Any, TypeId};
use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Bauanleitungen verschiedener Typen, je Typ eine; jede ist ein [`Recipe`] ihres Typs.
// Rc reicht, solange kein Testlisten-Punkt Threads verlangt; dann Arc.
type Services = HashMap<TypeId, Rc<dyn Any>>;

/// Dienste, die ein Scope schon gebaut hat, je Typ einer.
type Built = RefCell<HashMap<TypeId, Rc<dyn Any>>>;

/// Liefert bei jedem Aufruf einen Dienst vom Typ `T`, gebaut mit dem übergebenen Provider oder gemerkt in seinem Scope.
// dyn: `get` kennt nur `T`, nicht den Typ der Bauanleitung. Bauanleitungen verschiedener
// Typen (aus `instance`, `transient`, `singleton`, `scoped`) liegen deshalb hinter einem gemeinsamen Typ je `T`.
type Recipe<T> = Box<dyn Fn(&Provider) -> Rc<T>>;

/// Liefert Dienste nach ihrem Typ.
///
/// Entsteht über [`Provider::builder`]. Der Provider ist selbst ein Scope.
#[derive(Debug, Clone)]
pub struct Provider {
    services: Services,
    built: Rc<Built>,
    /// Der Speicher des Providers, aus dem alle Scopes stammen; dort liegen die `scoped`-Dienste eines Singletons.
    root: Rc<Built>,
    /// Typen, deren Bauanleitung gerade läuft; geteilt mit allen Scopes desselben Providers.
    building: Rc<RefCell<HashSet<TypeId>>>,
}

/// Sammelt Dienste und baut daraus einen [`Provider`].
///
/// Eine Bauanleitung bekommt den abfragenden Provider und holt darüber andere Dienste.
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

    /// Liefert den Dienst vom Typ `T`; `None`, wenn er fehlt oder seine Bauanleitung gerade läuft.
    #[must_use]
    pub fn get<T: 'static>(&self) -> Option<Rc<T>> {
        let id = TypeId::of::<T>();
        let recipe = self.services.get(&id)?.downcast_ref::<Recipe<T>>()?;
        let _building = Building::enter(&self.building, id)?;
        Some(recipe(self))
    }

    /// Beginnt einen neuen Scope mit denselben Diensten; `scoped`-Dienste baut er neu.
    #[must_use]
    pub fn scope(&self) -> Self {
        Self {
            services: self.services.clone(),
            built: Rc::default(),
            root: Rc::clone(&self.root),
            building: Rc::clone(&self.building),
        }
    }

    /// Liefert den Provider, aus dem dieser Scope stammt.
    fn root(&self) -> Self {
        Self {
            services: self.services.clone(),
            built: Rc::clone(&self.root),
            root: Rc::clone(&self.root),
            building: Rc::clone(&self.building),
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
    pub fn transient<T: 'static>(self, recipe: impl Fn(&Provider) -> T + 'static) -> Self {
        self.recipe(move |provider| Rc::new(recipe(provider)))
    }

    /// Registriert `recipe`; die erste Abfrage baut den Dienst mit dem Provider statt mit dem Scope, jede weitere liefert dasselbe Objekt.
    #[must_use]
    pub fn singleton<T: 'static>(self, recipe: impl Fn(&Provider) -> T + 'static) -> Self {
        let service = OnceCell::new();
        self.recipe(move |provider| {
            Rc::clone(service.get_or_init(|| Rc::new(recipe(&provider.root()))))
        })
    }

    /// Registriert `recipe`; die erste Abfrage in einem Scope baut den Dienst, jede weitere im selben Scope liefert dasselbe Objekt.
    #[must_use]
    pub fn scoped<T: 'static>(self, recipe: impl Fn(&Provider) -> T + 'static) -> Self {
        self.recipe(move |provider| scoped_service(provider, &recipe))
    }

    /// Baut den Provider mit allen registrierten Diensten.
    #[must_use]
    pub fn build(self) -> Provider {
        let built = Rc::<Built>::default();
        Provider {
            services: self.services,
            root: Rc::clone(&built),
            built,
            building: Rc::default(),
        }
    }

    fn recipe<T: 'static>(mut self, recipe: impl Fn(&Provider) -> Rc<T> + 'static) -> Self {
        let recipe: Recipe<T> = Box::new(recipe);
        self.services.insert(TypeId::of::<T>(), Rc::new(recipe));
        self
    }
}

/// Hält den Typ `id` in `types`, solange er lebt; gibt ihn beim Fallen frei, auch nach einem Panic der Bauanleitung.
struct Building<'a> {
    types: &'a RefCell<HashSet<TypeId>>,
    id: TypeId,
}

impl<'a> Building<'a> {
    /// Trägt `id` in `types` ein; `None`, wenn `id` dort schon steht.
    fn enter(types: &'a RefCell<HashSet<TypeId>>, id: TypeId) -> Option<Self> {
        if !types.borrow_mut().insert(id) {
            return None;
        }
        Some(Self { types, id })
    }
}

impl Drop for Building<'_> {
    fn drop(&mut self) {
        self.types.borrow_mut().remove(&self.id);
    }
}

/// Liefert den Dienst vom Typ `T` aus dem Scope von `provider`; fehlt er dort, baut `recipe` ihn, und der Scope merkt ihn sich.
fn scoped_service<T: 'static>(provider: &Provider, recipe: impl Fn(&Provider) -> T) -> Rc<T> {
    let id = TypeId::of::<T>();
    let cached = provider
        .built
        .borrow()
        .get(&id)
        .and_then(|service| Rc::clone(service).downcast().ok());
    cached.unwrap_or_else(|| {
        let service = Rc::new(recipe(provider));
        provider
            .built
            .borrow_mut()
            .insert(id, Rc::clone(&service) as Rc<dyn Any>);
        service
    })
}
