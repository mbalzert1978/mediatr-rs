//! Der Provider liefert Dienste, die sein Builder registriert hat.

mod builder;
mod registration;

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use registration::Recipe;

pub use builder::ProviderBuilder;
pub use registration::Registration;

/// Bauanleitungen verschiedener Typen, je Typ alle in der Reihenfolge ihrer Registrierung; jede ist ein [`Recipe`] ihres Typs.
// Rc reicht, solange kein Testlisten-Punkt Threads verlangt; dann Arc.
type Services = HashMap<TypeId, Vec<Rc<dyn Any>>>;

/// Dienste verschiedener Typen, die ein Scope schon gebaut hat, je `scoped`-Registrierung einer, nach ihrem [`Slot`].
type Built = RefCell<HashMap<Slot, Rc<dyn Any>>>;

/// Platz einer Registrierung: ihr Diensttyp und ihre Stelle in dessen Liste.
type Slot = (TypeId, usize);

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

impl Provider {
    /// Beginnt einen Provider ohne Dienste.
    #[must_use]
    pub fn builder() -> ProviderBuilder {
        ProviderBuilder::default()
    }

    /// Liefert den zuletzt registrierten Dienst vom Typ `T`; `None`, wenn er fehlt oder seine Bauanleitung gerade läuft.
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
