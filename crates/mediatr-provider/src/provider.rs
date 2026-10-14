//! Der Provider liefert Dienste, die sein Builder registriert hat.

use std::any::{Any, TypeId};
use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

/// Bauanleitungen verschiedener Typen, je Typ alle in der Reihenfolge ihrer Registrierung; jede ist ein [`Recipe`] ihres Typs.
// Rc reicht, solange kein Testlisten-Punkt Threads verlangt; dann Arc.
type Services = HashMap<TypeId, Vec<Rc<dyn Any>>>;

/// Dienste verschiedener Typen, die ein Scope schon gebaut hat, je `scoped`-Registrierung einer, nach ihrer Platznummer.
type Built = RefCell<HashMap<usize, Rc<dyn Any>>>;

/// Liefert bei jedem Aufruf einen Dienst vom Typ `T`, gebaut mit dem übergebenen Provider oder gemerkt in seinem Scope.
// dyn: `get` kennt nur `T`, nicht den Typ der Bauanleitung. Bauanleitungen verschiedener
// Typen (aus `instance`, `transient`, `singleton`, `scoped`) liegen deshalb hinter einem gemeinsamen Typ je `T`.
type Recipe<T> = Box<dyn Fn(&Provider) -> Rc<T>>;

/// Liefert Dienste nach ihrem Typ.
///
/// Entsteht über [`Provider::builder`]. Der Provider ist selbst ein Scope.
#[derive(Debug, Clone)]
pub struct Provider {
    shared: Rc<Shared>,
    built: Rc<Built>,
}

/// Was ein Provider mit allen seinen Scopes teilt.
#[derive(Debug)]
struct Shared {
    services: Services,
    /// Der Speicher des Providers, aus dem alle Scopes stammen; dort liegen die `scoped`-Dienste eines Singletons.
    root: Rc<Built>,
    /// Typen, deren Bauanleitung gerade läuft.
    building: RefCell<HashSet<TypeId>>,
}

/// Verweis auf einen [`Provider`], den [`Provider::downgrade`] liefert; hält dessen Dienste nicht am Leben.
///
/// [`WeakProvider::upgrade`] macht daraus wieder einen Provider.
#[derive(Debug, Clone)]
pub struct WeakProvider {
    shared: Weak<Shared>,
    built: Weak<Built>,
}

/// Sammelt Dienste und baut daraus einen [`Provider`].
///
/// Eine Bauanleitung bekommt den abfragenden Provider und holt darüber andere Dienste.
#[derive(Debug, Clone, Default)]
pub struct ProviderBuilder {
    services: Services,
    /// Paare aus Diensttyp und Quelltyp, die [`ProviderBuilder::unique_instance`] schon registriert hat.
    sources: HashSet<(TypeId, TypeId)>,
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
        let recipe = self
            .shared
            .services
            .get(&id)?
            .last()?
            .downcast_ref::<Recipe<T>>()?;
        let _building = Building::enter(&self.shared.building, id)?;
        Some(recipe(self))
    }

    /// Liefert alle Dienste vom Typ `T`, in der Reihenfolge ihrer Registrierung; keinen, wenn eine Bauanleitung von `T` gerade läuft.
    #[must_use]
    pub fn all<T: 'static>(&self) -> Vec<Rc<T>> {
        let id = TypeId::of::<T>();
        let Some(_building) = Building::enter(&self.shared.building, id) else {
            return Vec::new();
        };
        self.shared
            .services
            .get(&id)
            .into_iter()
            .flatten()
            .filter_map(|recipe| recipe.downcast_ref::<Recipe<T>>())
            .map(|recipe| recipe(self))
            .collect()
    }

    /// Beginnt einen neuen Scope mit denselben Diensten; `scoped`-Dienste baut er neu.
    #[must_use]
    pub fn scope(&self) -> Self {
        Self {
            shared: Rc::clone(&self.shared),
            built: Rc::default(),
        }
    }

    /// Liefert einen Verweis auf diesen Provider, den eine Bauanleitung sich merken kann.
    #[must_use]
    pub fn downgrade(&self) -> WeakProvider {
        WeakProvider {
            shared: Rc::downgrade(&self.shared),
            built: Rc::downgrade(&self.built),
        }
    }

    /// Liefert den Provider, aus dem dieser Scope stammt.
    fn root(&self) -> Self {
        Self {
            shared: Rc::clone(&self.shared),
            built: Rc::clone(&self.shared.root),
        }
    }
}

impl WeakProvider {
    /// Liefert den Provider oder Scope, auf den dieser Verweis zeigt; `None`, wenn er weggefallen ist.
    #[must_use]
    pub fn upgrade(&self) -> Option<Provider> {
        Some(Provider {
            shared: self.shared.upgrade()?,
            built: self.built.upgrade()?,
        })
    }
}

impl ProviderBuilder {
    /// Registriert `service`; jede Abfrage liefert dasselbe Objekt.
    #[must_use]
    pub fn instance<T: 'static>(self, service: T) -> Self {
        let service = Rc::new(service);
        self.recipe(move |_| Rc::clone(&service))
    }

    /// Registriert `into(source)` wie [`instance`](Self::instance); hat `unique_instance` für `T` schon einen Dienst aus einem `S` registriert, fällt dieser weg.
    #[must_use]
    pub fn unique_instance<S: 'static, T: 'static>(
        mut self,
        source: S,
        into: impl FnOnce(S) -> T,
    ) -> Self {
        if !self.sources.insert((TypeId::of::<T>(), TypeId::of::<S>())) {
            return self;
        }
        self.instance(into(source))
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
        // Die Zahl der Registrierungen davor ist für jede Registrierung eine andere Platznummer.
        let slot = self.services.values().map(Vec::len).sum();
        self.recipe(move |provider| scoped_service(provider, slot, &recipe))
    }

    /// Baut den Provider mit allen registrierten Diensten.
    #[must_use]
    pub fn build(self) -> Provider {
        let root = Rc::<Built>::default();
        Provider {
            built: Rc::clone(&root),
            shared: Rc::new(Shared {
                services: self.services,
                root,
                building: RefCell::default(),
            }),
        }
    }

    fn recipe<T: 'static>(mut self, recipe: impl Fn(&Provider) -> Rc<T> + 'static) -> Self {
        let recipe: Recipe<T> = Box::new(recipe);
        self.services
            .entry(TypeId::of::<T>())
            .or_default()
            .push(Rc::new(recipe));
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

/// Liefert den Dienst auf Platz `slot` im Scope von `provider`; fehlt er dort, baut `recipe` ihn, und der Scope merkt ihn sich.
fn scoped_service<T: 'static>(
    provider: &Provider,
    slot: usize,
    recipe: impl Fn(&Provider) -> T,
) -> Rc<T> {
    let cached = provider
        .built
        .borrow()
        .get(&slot)
        .and_then(|service| Rc::clone(service).downcast().ok());
    cached.unwrap_or_else(|| {
        let service = Rc::new(recipe(provider));
        provider
            .built
            .borrow_mut()
            .insert(slot, Rc::clone(&service) as Rc<dyn Any>);
        service
    })
}
