//! Library surface for the `dry-py` Python adapter.

#![forbid(unsafe_code)]

pub mod normalize;
pub mod runner;

#[doc(inline)]
pub use normalize::PyNormalizer;
