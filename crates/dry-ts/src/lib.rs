//! Library surface for the `dry-ts` TypeScript adapter.

#![forbid(unsafe_code)]

pub mod normalize;
pub mod runner;

#[doc(inline)]
pub use normalize::TsNormalizer;
