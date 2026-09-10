//! Ports that language adapters implement.

mod normalizer;

#[doc(inline)]
pub use normalizer::{LanguageNormalizer, NormalizeError, NormalizeOutcome};
