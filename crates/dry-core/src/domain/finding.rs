//! Finding and member location types.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{CloneType, FormSpan, Tier};

/// One member of a clone finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormMember {
    /// Source file path.
    pub path: PathBuf,
    /// Inclusive start line.
    pub start_line: u32,
    /// Inclusive end line.
    pub end_line: u32,
    /// Form display name.
    pub name: String,
}

impl FormMember {
    /// Builds a member from path, span, and name.
    #[must_use]
    pub const fn new(path: PathBuf, span: FormSpan, name: String) -> Self {
        Self {
            path,
            start_line: span.start_line,
            end_line: span.end_line,
            name,
        }
    }
}

/// A scored structural duplication group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// Clone taxonomy label.
    pub clone_type: CloneType,
    /// Agentic routing tier.
    pub tier: Tier,
    /// Jaccard similarity in `[0.0, 1.0]`.
    pub score: f64,
    /// Participating form locations (two or more).
    pub members: Vec<FormMember>,
}
