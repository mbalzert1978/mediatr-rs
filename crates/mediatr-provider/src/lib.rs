//! Baut Dienste nach der Lebensdauer, mit der sie registriert sind.

mod error;
mod provider;

pub use error::GetError;
pub use provider::{Provider, ProviderBuilder, Registration, WeakProvider};
