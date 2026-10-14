//! Der Provider liefert Dienste, die sein Builder registriert hat.

use std::any::{Any, TypeId};
use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::{Rc, Weak};

/// Bauanleitungen verschiedener Typen, je Typ alle in der Reihenfolge ihrer Registrierung; jede ist ein [`Recipe`] ihres Typs.
// Rc reicht, solange kein Testlisten-Punkt Threads verlangt; dann Arc.
type Services = HashMap<TypeId, Vec<Rc<dyn Any>>>;

/// Dienste verschiedener Typen, die ein Scope schon gebaut hat, je `scoped`-Registrierung einer, nach ihrem [`Slot`].
type Built = RefCell<HashMap<Slot, Rc<dyn Any>>>;

/// Platz einer Registrierung: ihr Diensttyp und ihre Stelle in dessen Liste.
type Slot = (TypeId, usize);

/// Liefert bei jedem Aufruf einen Dienst vom Typ `T`, gebaut mit dem übergebenen Provider oder gemerkt in seinem Scope auf dem übergebenen Platz.
// dyn: `get` kennt nur `T`, nicht den Typ der Bauanleitung. Bauanleitungen verschiedener
// Typen (aus `Registration::instance`, `transient`, `singleton`, `scoped`) liegen deshalb hinter einem gemeinsamen Typ je `T`.
type Recipe<T> = Box<dyn Fn(&Provider, Slot) -> Rc<T>>;

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
    /// Paare aus Diensttyp und Schlüsseltyp, die [`ProviderBuilder::register`] als einmalig schon aufgenommen hat.
    keys: HashSet<(TypeId, TypeId)>,
}

/// Wie ein Dienst vom Typ `T` entsteht; [`ProviderBuilder::register`] nimmt ihn auf.
pub struct Registration<T> {
    recipe: Recipe<T>,
    /// Schlüsseltyp aus [`Registration::unique_by`]; `None` heißt: nicht einmalig.
    key: Option<TypeId>,
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
        let (index, recipe) = self
            .shared
            .services
            .get(&id)?
            .iter()
            .enumerate()
            .next_back()?;
        let recipe = recipe.downcast_ref::<Recipe<T>>()?;
        let _building = Building::enter(&self.shared.building, id)?;
        Some(recipe(self, (id, index)))
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
            .enumerate()
            .filter_map(|(index, recipe)| Some((index, recipe.downcast_ref::<Recipe<T>>()?)))
            .map(|(index, recipe)| recipe(self, (id, index)))
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

impl<T: 'static> Registration<T> {
    /// Dienst `service`; jede Abfrage liefert dasselbe Objekt.
    #[must_use]
    pub fn instance(service: T) -> Self {
        let service = Rc::new(service);
        Self::new(move |_, _| Rc::clone(&service))
    }

    /// Dienst aus `recipe`; jede Abfrage baut ihn damit neu.
    #[must_use]
    pub fn transient(recipe: impl Fn(&Provider) -> T + 'static) -> Self {
        Self::new(move |provider, _| Rc::new(recipe(provider)))
    }

    /// Dienst aus `recipe`; die erste Abfrage baut ihn mit dem Provider statt mit dem Scope, jede weitere liefert dasselbe Objekt.
    #[must_use]
    pub fn singleton(recipe: impl Fn(&Provider) -> T + 'static) -> Self {
        let service = OnceCell::new();
        Self::new(move |provider, _| {
            Rc::clone(service.get_or_init(|| Rc::new(recipe(&provider.root()))))
        })
    }

    /// Dienst aus `recipe`; die erste Abfrage in einem Scope baut ihn, jede weitere im selben Scope liefert dasselbe Objekt.
    #[must_use]
    pub fn scoped(recipe: impl Fn(&Provider) -> T + 'static) -> Self {
        Self::new(move |provider, slot| scoped_service(provider, slot, &recipe))
    }

    /// Macht die Registrierung einmalig je Schlüsseltyp `K`: Hat [`ProviderBuilder::register`] für `T` schon eine Registrierung mit `K` aufgenommen, nimmt es diese nicht mehr auf.
    #[must_use]
    pub fn unique_by<K: 'static>(self) -> Self {
        Self {
            key: Some(TypeId::of::<K>()),
            ..self
        }
    }

    fn new(recipe: impl Fn(&Provider, Slot) -> Rc<T> + 'static) -> Self {
        Self {
            recipe: Box::new(recipe),
            key: None,
        }
    }
}

impl<T> fmt::Debug for Registration<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registration")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

impl ProviderBuilder {
    /// Nimmt `registration` auf; ist sie einmalig und für ihren Typ schon eine mit demselben Schlüsseltyp aufgenommen, nimmt es sie nicht auf.
    #[must_use]
    pub fn register<T: 'static>(mut self, registration: Registration<T>) -> Self {
        let id = TypeId::of::<T>();
        if let Some(key) = registration.key
            && !self.keys.insert((id, key))
        {
            return self;
        }
        self.services
            .entry(id)
            .or_default()
            .push(Rc::new(registration.recipe));
        self
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
    slot: Slot,
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
