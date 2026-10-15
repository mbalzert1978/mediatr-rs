//! Wie ein Dienst entsteht.

use std::any::{Any, TypeId, type_name};
use std::fmt;
use std::sync::Arc;
#[expect(
    clippy::disallowed_types,
    reason = "lokaler Wert einer Bauanleitung, kein globaler Zustand"
)]
use std::sync::OnceLock;

use super::{Slot, lock};
use crate::Provider;

/// Liefert bei jedem Aufruf einen Dienst vom Typ `T`: neu gebaut mit dem übergebenen Provider, einmal angelegt und in der Bauanleitung gemerkt (`instance`, `singleton`), oder im Scope des übergebenen Providers auf dem übergebenen Platz gemerkt (`scoped`).
// dyn: `get` kennt nur `T`, nicht den Typ der Bauanleitung. Bauanleitungen verschiedener
// Typen (aus `Registration::instance`, `transient`, `singleton`, `scoped`, `map`) liegen deshalb hinter einem gemeinsamen Typ je `T`.
pub(super) type Recipe<T> = Box<dyn Fn(&Provider, Slot) -> Arc<T> + Send + Sync>;

/// Wie ein Dienst vom Typ `T` entsteht; [`ProviderBuilder::register`](crate::ProviderBuilder::register) nimmt ihn auf.
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
        Self::new(move |_, _| Arc::clone(&service))
    }

    /// Dienst aus `recipe`; jede Abfrage baut ihn damit neu.
    #[must_use]
    pub fn transient(recipe: impl Fn(&Provider) -> T + Send + Sync + 'static) -> Self {
        Self::new(move |provider, _| Arc::new(recipe(provider)))
    }

    /// Dienst aus `recipe`; die erste Abfrage baut ihn mit dem Provider statt mit dem Scope, jede weitere liefert dasselbe Objekt.
    ///
    /// Singleton-Bauanleitungen dürfen einander nicht abfragen: Starten zwei Threads gleichzeitig je eine davon, warten beide für immer.
    #[must_use]
    #[expect(
        clippy::disallowed_types,
        reason = "lokaler Wert einer Bauanleitung, kein globaler Zustand"
    )]
    pub fn singleton(recipe: impl Fn(&Provider) -> T + Send + Sync + 'static) -> Self {
        let service = OnceLock::new();
        Self::new(move |provider, _| {
            Arc::clone(service.get_or_init(|| Arc::new(recipe(&provider.root()))))
        })
    }

    /// Dienst aus `recipe`; die erste Abfrage in einem Scope baut ihn, jede weitere im selben Scope liefert dasselbe Objekt.
    ///
    /// Fragen zwei Threads im selben Scope gleichzeitig ab, kann die Bauanleitung zweimal laufen; beide bekommen dasselbe Objekt.
    #[must_use]
    pub fn scoped(recipe: impl Fn(&Provider) -> T + Send + Sync + 'static) -> Self {
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
            ..Registration::new(move |provider, slot| Arc::new(f(recipe(provider, slot))))
        }
    }

    fn new(recipe: impl Fn(&Provider, Slot) -> Arc<T> + Send + Sync + 'static) -> Self {
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

/// Liefert den Dienst auf Platz `slot` im Scope von `provider`; fehlt er dort, baut `recipe` ihn, und der Scope merkt ihn sich.
///
/// Bauen zwei Threads denselben Platz, bleibt der Dienst des ersten; der zweite verwirft seinen Bau.
fn scoped_service<T: Send + Sync + 'static>(
    provider: &Provider,
    slot: Slot,
    recipe: impl Fn(&Provider) -> T,
) -> Arc<T> {
    let cached = lock(&provider.built)
        .get(&slot)
        .and_then(|service| Arc::clone(service).downcast().ok());
    cached.unwrap_or_else(|| {
        let service = Arc::new(recipe(provider));
        let kept = Arc::clone(
            lock(&provider.built)
                .entry(slot)
                .or_insert(Arc::clone(&service) as Arc<dyn Any + Send + Sync>),
        );
        kept.downcast().unwrap_or(service)
    })
}
