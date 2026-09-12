//! Language-agnostic normalized trees and subtree fingerprinting.
//!
//! Adapters emit [`NormNode`] trees; comparison consumes fingerprint sets.

mod fingerprint;
mod tree;

#[doc(inline)]
pub use fingerprint::{FingerprintResult, fingerprint_tree};
#[doc(inline)]
pub use tree::NormNode;
