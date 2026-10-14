//! Baut Dienste nach der Lebensdauer, mit der sie registriert sind.

mod provider;

pub use provider::{Provider, ProviderBuilder, Registration, WeakProvider};
