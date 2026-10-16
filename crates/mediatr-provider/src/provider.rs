//! Der Provider liefert Dienste, die sein Builder registriert hat.

mod builder;
mod registration;

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::thread::{self, ThreadId};

use crate::error::GetError;
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
/// Entsteht über [`Provider::builder`]. `scoped`-Dienste liefert nur ein Scope aus [`Provider::scope`].
#[derive(Debug, Clone)]
pub struct Provider {
    shared: Arc<Shared>,
    built: Arc<Built>,
}

/// Was ein Provider mit allen seinen Scopes teilt.
#[derive(Debug)]
struct Shared {
    services: Services,
    /// Der Speicher des Providers, aus dem alle Scopes stammen; daran erkennt der Provider, ob er selbst oder ein Scope abgefragt wird.
    root: Arc<Built>,
    /// Typen, deren Bauanleitung gerade läuft, je mit dem Thread, auf dem sie läuft.
    building: Mutex<HashSet<(ThreadId, TypeId)>>,
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

    /// Liefert den zuletzt registrierten Dienst vom Typ `T`; sonst den [`GetError`], der sagt, warum nicht.
    ///
    /// # Errors
    ///
    /// Liefert [`GetError::NoService`], wenn kein Dienst vom Typ `T` registriert ist,
    /// [`GetError::Cycle`], wenn seine Bauanleitung auf diesem Thread gerade läuft,
    /// [`GetError::ScopedFromRoot`], wenn ein `scoped`-Dienst beim Provider selbst abgefragt wird,
    /// und [`GetError::NoDependency`], wenn seine Bauanleitung einen nicht registrierten Dienst abfragt und den Fehler weitergibt.
    pub fn get<T: 'static>(&self) -> Result<Arc<T>, GetError> {
        let id = TypeId::of::<T>();
        let (index, recipe) = self
            .shared
            .services
            .get(&id)
            .and_then(|recipes| recipes.iter().enumerate().next_back())
            .ok_or(GetError::NoService)?;
        let recipe = recipe
            .downcast_ref::<Recipe<T>>()
            .ok_or(GetError::NoService)?;
        let _building = Building::enter(&self.shared.building, id).ok_or(GetError::Cycle)?;
        recipe(self, (id, index)).map_err(|error| match error {
            GetError::NoService => GetError::NoDependency,
            other => other,
        })
    }

    /// Liefert alle Dienste vom Typ `T`, in der Reihenfolge ihrer Registrierung, eine leere Liste, wenn keiner registriert ist; sonst den [`GetError`], der sagt, warum nicht.
    ///
    /// # Errors
    ///
    /// Liefert [`GetError::Cycle`], wenn die Bauanleitung von `T` auf diesem Thread gerade läuft,
    /// [`GetError::ScopedFromRoot`], wenn ein `scoped`-Dienst von `T` beim Provider selbst abgefragt wird,
    /// und [`GetError::NoDependency`], wenn eine Bauanleitung einen nicht registrierten Dienst abfragt und den Fehler weitergibt.
    pub fn all<T: 'static>(&self) -> Result<Vec<Arc<T>>, GetError> {
        let id = TypeId::of::<T>();
        let _building = Building::enter(&self.shared.building, id).ok_or(GetError::Cycle)?;
        self.shared
            .services
            .get(&id)
            .into_iter()
            .flatten()
            .enumerate()
            .filter_map(|(index, recipe)| Some((index, recipe.downcast_ref::<Recipe<T>>()?)))
            .map(|(index, recipe)| recipe(self, (id, index)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| match error {
                GetError::NoService => GetError::NoDependency,
                other => other,
            })
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

    /// Ob dies der Provider selbst ist und kein Scope aus [`Provider::scope`].
    fn is_root(&self) -> bool {
        Arc::ptr_eq(&self.built, &self.shared.root)
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

/// Hält den Typ `id` für den laufenden Thread in `types`, solange er lebt; gibt ihn beim Fallen frei, auch nach einem Panic der Bauanleitung.
struct Building<'a> {
    types: &'a Mutex<HashSet<(ThreadId, TypeId)>>,
    key: (ThreadId, TypeId),
}

impl<'a> Building<'a> {
    /// Trägt `id` für den laufenden Thread in `types` ein; `None`, wenn er dort schon steht.
    fn enter(types: &'a Mutex<HashSet<(ThreadId, TypeId)>>, id: TypeId) -> Option<Self> {
        let key = (thread::current().id(), id);
        if !lock(types).insert(key) {
            return None;
        }
        Some(Self { types, key })
    }
}

impl Drop for Building<'_> {
    fn drop(&mut self) {
        lock(self.types).remove(&self.key);
    }
}
