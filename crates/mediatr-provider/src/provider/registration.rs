//! Wie ein Dienst entsteht.

use std::any::{Any, TypeId, type_name};
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError, RwLock, TryLockError};

use super::{Owning, Slot, Waiting, lock};
use crate::{GetError, Provider};

/// Liefert bei jedem Aufruf einen Dienst vom Typ `T`: neu gebaut mit dem übergebenen Provider, einmal angelegt und in der Bauanleitung gemerkt (`instance`, `singleton`), oder im Scope des übergebenen Providers auf dem übergebenen Platz gemerkt (`scoped`).
// dyn: `get` kennt nur `T`, nicht den Typ der Bauanleitung. Bauanleitungen verschiedener
// Typen (aus `Registration::instance`, `transient`, `singleton`, `scoped`, `map`) liegen deshalb hinter einem gemeinsamen Typ je `T`.
pub(super) type Recipe<T> = Box<dyn Fn(&Provider, Slot) -> Result<Arc<T>, GetError> + Send + Sync>;

/// Wie ein Dienst vom Typ `T` entsteht; [`ProviderBuilder::register`](crate::ProviderBuilder::register) nimmt ihn auf.
///
/// Liefert die Bauanleitung von `transient`, `singleton` oder `scoped` einen Fehler, liefert die Abfrage ihn weiter; aus `Err(GetError::NoService)` wird dabei `Err(GetError::NoDependency)`.
pub struct Registration<T> {
    pub(super) recipe: Recipe<T>,
    /// Schlüsseltyp aus [`Registration::unique_by`]; `None` heißt: nicht einmalig.
    pub(super) key: Option<TypeId>,
}

impl<T: Send + Sync + 'static> Registration<T> {
    /// Dienst `service`; jede Abfrage liefert dasselbe Objekt.
    #[must_use]
    pub fn instance(service: T) -> Self {
        let service = Arc::new(service);
        Self::new(move |_, _| Ok(Arc::clone(&service)))
    }

    /// Dienst aus `recipe`; jede Abfrage baut ihn damit neu.
    #[must_use]
    pub fn transient(
        recipe: impl Fn(&Provider) -> Result<T, GetError> + Send + Sync + 'static,
    ) -> Self {
        Self::new(move |provider, slot| provider.run_recipe(slot.0, &recipe))
    }

    /// Dienst aus `recipe`; die erste Abfrage baut ihn mit dem Provider statt mit dem Scope, jede weitere liefert dasselbe Objekt.
    ///
    /// Fragt die Bauanleitung nach ihrem eigenen Typ, bekommt sie `Err(GetError::Cycle)`. Fragt sie nach einem `scoped`-Dienst, bekommt sie `Err(GetError::ScopedFromRoot)`, denn sie baut mit dem Provider. Brauchen zwei Singletons einander und bauen zwei Threads sie gleichzeitig, bekommt eine der beiden inneren Abfragen `Err(GetError::Cycle)`.
    /// Schlägt `recipe` fehl, merkt sich die Bauanleitung nichts, und die nächste Abfrage baut neu.
    #[must_use]
    pub fn singleton(
        recipe: impl Fn(&Provider) -> Result<T, GetError> + Send + Sync + 'static,
    ) -> Self {
        let kept = Singleton {
            service: RwLock::new(None),
            building: Mutex::new(()),
        };
        Self::new(move |provider, slot| kept.service(provider, slot, &recipe))
    }

    /// Dienst aus `recipe`; die erste Abfrage in einem Scope baut ihn, jede weitere im selben Scope liefert dasselbe Objekt.
    ///
    /// Beim Provider selbst abgefragt, liefert sie `Err(GetError::ScopedFromRoot)`; nur ein Scope aus [`Provider::scope`] baut den Dienst.
    ///
    /// Fragen zwei Threads im selben Scope gleichzeitig ab, kann die Bauanleitung zweimal laufen; beide bekommen dasselbe Objekt.
    #[must_use]
    pub fn scoped(
        recipe: impl Fn(&Provider) -> Result<T, GetError> + Send + Sync + 'static,
    ) -> Self {
        Self::new(move |provider, slot| scoped_service(provider, slot, &recipe))
    }

    /// Macht die Registrierung einmalig je Schlüsseltyp `K`: Hat [`ProviderBuilder::register`](crate::ProviderBuilder::register) für `T` schon eine Registrierung mit `K` aufgenommen, nimmt es diese nicht mehr auf.
    #[must_use]
    pub fn unique_by<K: 'static>(self) -> Self {
        Self {
            key: Some(TypeId::of::<K>()),
            ..self
        }
    }

    /// Dienst `f(dienst)` vom Typ `U` statt des Diensts vom Typ `T`; den Dienst vom Typ `T` baut die Bauart wie zuvor, `f` läuft bei jeder Abfrage neu.
    ///
    /// Ein Schlüsseltyp aus [`Registration::unique_by`] bleibt.
    #[must_use]
    pub fn map<U: Send + Sync + 'static>(
        self,
        f: impl Fn(Arc<T>) -> U + Send + Sync + 'static,
    ) -> Registration<U> {
        let Self { recipe, key } = self;
        Registration {
            key,
            ..Registration::new(move |provider, slot| Ok(Arc::new(f(recipe(provider, slot)?))))
        }
    }

    fn new(
        recipe: impl Fn(&Provider, Slot) -> Result<Arc<T>, GetError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            recipe: Box::new(recipe),
            key: None,
        }
    }
}

impl<T> fmt::Debug for Registration<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registration")
            .field("service", &type_name::<T>())
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

/// Das Singleton einer `singleton`-Registrierung; leer, bis die erste Abfrage es baut.
struct Singleton<T> {
    /// Das fertige Singleton; Abfragen auf mehreren Threads lesen es gleichzeitig, ohne aufeinander zu warten.
    service: RwLock<Option<Arc<T>>>,
    /// Gesperrt, solange ein Thread das Singleton baut, damit der Bau nur einmal läuft.
    building: Mutex<()>,
}

impl<T> Singleton<T> {
    /// Das fertige Singleton; `None`, solange es fehlt.
    fn built(&self) -> Option<Arc<T>> {
        self.service
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Liefert das fertige Singleton; fehlt es, baut `recipe` es mit dem Provider statt mit dem Scope von `provider`, und es bleibt gemerkt. Schlägt der Bau fehl, bleibt das Singleton leer.
    ///
    /// Baut ein anderer Thread gerade, wartet dieser Thread auf ihn; führt die Kette der Wartenden zurück zu diesem Thread, liefert sie `Err(GetError::Cycle)` statt zu warten.
    fn service(
        &self,
        provider: &Provider,
        slot: Slot,
        recipe: impl Fn(&Provider) -> Result<T, GetError>,
    ) -> Result<Arc<T>, GetError> {
        if let Some(service) = self.built() {
            return Ok(service);
        }
        let _building = match self.building.try_lock() {
            Ok(building) => building,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => {
                let _waiting = Waiting::enter(&provider.shared.builds, slot)?;
                lock(&self.building)
            }
        };
        if let Some(service) = self.built() {
            return Ok(service);
        }
        let _owning = Owning::enter(&provider.shared.builds, slot);
        let built = provider.root().run_recipe(slot.0, recipe)?;
        *self.service.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(&built));
        Ok(built)
    }
}

/// Liefert den Dienst auf Platz `slot` im Scope von `provider`; fehlt er dort, baut `recipe` ihn, und der Scope merkt ihn sich. Ist `provider` der Provider selbst, liefert sie `Err(GetError::ScopedFromRoot)`.
///
/// Bauen zwei Threads denselben Platz, bleibt der Dienst des ersten; der zweite verwirft seinen Bau.
fn scoped_service<T: Send + Sync + 'static>(
    provider: &Provider,
    slot: Slot,
    recipe: impl Fn(&Provider) -> Result<T, GetError>,
) -> Result<Arc<T>, GetError> {
    if provider.is_root() {
        return Err(GetError::ScopedFromRoot);
    }
    let cached = lock(&provider.built)
        .get(&slot)
        .and_then(|service| Arc::clone(service).downcast().ok());
    if let Some(service) = cached {
        return Ok(service);
    }
    let service = provider.run_recipe(slot.0, recipe)?;
    let kept = Arc::clone(
        lock(&provider.built)
            .entry(slot)
            .or_insert(Arc::clone(&service) as Arc<dyn Any + Send + Sync>),
    );
    Ok(kept.downcast().unwrap_or(service))
}
