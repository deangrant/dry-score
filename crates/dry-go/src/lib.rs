//! Library surface for the `dry-go` Go adapter.

#![forbid(unsafe_code)]

pub mod normalize;
pub mod runner;

#[doc(inline)]
pub use normalize::GoNormalizer;
