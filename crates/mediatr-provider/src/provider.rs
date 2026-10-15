//! Der Provider liefert Dienste, die sein Builder registriert hat.

mod builder;
mod registration;

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use registration::Recipe;

pub use builder::ProviderBuilder;
pub use registration::Registration;

/// Bauanleitungen verschiedener Typen, je Typ alle in der Reihenfolge ihrer Registrierung; jede ist ein [`Recipe`] ihres Typs.
type Services = HashMap<TypeId, Vec<Arc<dyn Any + Send + Sync>>>;

/// Dienste verschiedener Typen, die ein Scope schon gebaut hat, je `scoped`-Registrierung einer, nach ihrem [`Slot`].
type Built = Mutex<HashMap<Slot, Arc<dyn Any + Send + Sync>>>;

/// Platz einer Registrierung: ihr Diensttyp und ihre Stelle in dessen Liste.
type Slot = (TypeId, usize);

/// Liefert Dienste nach ihrem Typ.
///
/// Entsteht über [`Provider::builder`]. Der Provider ist selbst ein Scope.
#[derive(Debug, Clone)]
pub struct Provider {
    shared: Arc<Shared>,
    built: Arc<Built>,
}

/// Was ein Provider mit allen seinen Scopes teilt.
#[derive(Debug)]
struct Shared {
    services: Services,
    /// Der Speicher des Providers, aus dem alle Scopes stammen; dort liegen die `scoped`-Dienste eines Singletons.
    root: Arc<Built>,
    /// Typen, deren Bauanleitung gerade läuft.
    building: Mutex<HashSet<TypeId>>,
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
    pub fn get<T: 'static>(&self) -> Option<Arc<T>> {
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
    pub fn all<T: 'static>(&self) -> Vec<Arc<T>> {
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
            shared: Arc::clone(&self.shared),
            built: Arc::default(),
        }
    }

    /// Liefert einen Verweis auf diesen Provider, den eine Bauanleitung sich merken kann.
    #[must_use]
    pub fn downgrade(&self) -> WeakProvider {
        WeakProvider {
            shared: Arc::downgrade(&self.shared),
            built: Arc::downgrade(&self.built),
        }
    }

    /// Liefert den Provider, aus dem dieser Scope stammt.
    fn root(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
            built: Arc::clone(&self.shared.root),
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

/// Sperrt `mutex`; ein Panic in einer Bauanleitung macht den Provider nicht unbrauchbar.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Hält den Typ `id` in `types`, solange er lebt; gibt ihn beim Fallen frei, auch nach einem Panic der Bauanleitung.
struct Building<'a> {
    types: &'a Mutex<HashSet<TypeId>>,
    id: TypeId,
}

impl<'a> Building<'a> {
    /// Trägt `id` in `types` ein; `None`, wenn `id` dort schon steht.
    fn enter(types: &'a Mutex<HashSet<TypeId>>, id: TypeId) -> Option<Self> {
        if !lock(types).insert(id) {
            return None;
        }
        Some(Self { types, id })
    }
}

impl Drop for Building<'_> {
    fn drop(&mut self) {
        lock(self.types).remove(&self.id);
    }
}
