//! Library surface for the `dry-rs` Rust adapter.

#![forbid(unsafe_code)]

pub mod cli;
pub mod normalize;
pub mod runner;

#[doc(inline)]
pub use normalize::RustNormalizer;
