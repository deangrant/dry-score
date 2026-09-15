//! Clone-type and tier classification from score signals.

use crate::domain::{CloneType, Tier};

/// Score floor for [`Tier::AutoRefactor`].
pub const AUTO_REFACTOR_FLOOR: f64 = 0.95;

/// Score floor for [`Tier::ReviewFirst`].
pub const REVIEW_FIRST_FLOOR: f64 = 0.85;

/// Maps similarity and identifier equality to a clone taxonomy label.
#[must_use]
pub fn classify(score: f64, idents_identical: bool) -> CloneType {
    if (score - 1.0).abs() > f64::EPSILON {
        return CloneType::Type3;
    }
    if idents_identical {
        CloneType::Type1
    } else {
        CloneType::Type2
    }
}

/// Maps similarity to an agentic routing tier.
///
/// Bands: auto ≥ 0.95, review ≥ 0.85, else advisory. The advisory band for
/// emitted findings is further gated by the compare threshold at emission time
/// (empty when that threshold ≥ 0.85).
#[must_use]
pub fn tier_for(score: f64) -> Tier {
    if score >= AUTO_REFACTOR_FLOOR {
        Tier::AutoRefactor
    } else if score >= REVIEW_FIRST_FLOOR {
        Tier::ReviewFirst
    } else {
        Tier::Advisory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_exact_and_near_miss() {
        let cases = [
            (1.0, true, CloneType::Type1),
            (1.0, false, CloneType::Type2),
            (0.9, true, CloneType::Type3),
        ];
        for (score, idents, expected) in cases {
            assert_eq!(classify(score, idents), expected);
        }
    }

    #[test]
    fn tier_floors() {
        assert_eq!(tier_for(0.99), Tier::AutoRefactor);
        assert_eq!(tier_for(0.9), Tier::ReviewFirst);
        assert_eq!(tier_for(0.82), Tier::Advisory);
        assert_eq!(tier_for(0.5), Tier::Advisory);
    }
}
