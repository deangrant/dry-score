//! Closed enums for clone classification and agentic routing.

use serde::{Deserialize, Serialize};

/// Kind of source form relative to tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormKind {
    /// Ordinary production code.
    Production,
    /// Test harness code (`#[test]` / `#[cfg(test)]`).
    Test,
}

impl FormKind {
    /// Stable wire label for this kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        // dry-rs:ignore. Per-enum vocabulary; shared match shape is intentional.
        match self {
            Self::Production => "production",
            Self::Test => "test",
        }
    }
}

/// Conventional clone taxonomy label for a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloneType {
    /// Exact structure after trivia removal.
    Type1,
    /// Same structure with renamed identifiers or abstracted literals.
    Type2,
    /// Near-miss: shared structure with statement-level drift.
    Type3,
}

impl CloneType {
    /// Stable wire label for this clone type.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        // dry-rs:ignore. Per-enum vocabulary; shared match shape is intentional.
        match self {
            Self::Type1 => "type_1",
            Self::Type2 => "type_2",
            Self::Type3 => "type_3",
        }
    }
}

/// Agentic routing tier derived from similarity score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// Score ≥ 0.95; strong candidate for automated refactor.
    AutoRefactor,
    /// Score ≥ 0.85; propose refactor, review before merge.
    ReviewFirst,
    /// Score above threshold but below 0.85.
    Advisory,
}

impl Tier {
    /// Stable wire label for this tier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        // dry-rs:ignore. Per-enum vocabulary; shared match shape is intentional.
        match self {
            Self::AutoRefactor => "auto_refactor",
            Self::ReviewFirst => "review_first",
            Self::Advisory => "advisory",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_kind_as_str() {
        assert_eq!(FormKind::Production.as_str(), "production");
        assert_eq!(FormKind::Test.as_str(), "test");
    }

    #[test]
    fn clone_type_as_str() {
        assert_eq!(CloneType::Type1.as_str(), "type_1");
        assert_eq!(CloneType::Type2.as_str(), "type_2");
        assert_eq!(CloneType::Type3.as_str(), "type_3");
    }

    #[test]
    fn tier_as_str() {
        assert_eq!(Tier::AutoRefactor.as_str(), "auto_refactor");
        assert_eq!(Tier::ReviewFirst.as_str(), "review_first");
        assert_eq!(Tier::Advisory.as_str(), "advisory");
    }
}
