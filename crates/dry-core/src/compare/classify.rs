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
#[must_use]
pub fn tier_for(score: f64, _threshold: f64) -> Tier {
    if score >= AUTO_REFACTOR_FLOOR {
        return Tier::AutoRefactor;
    }
    if score >= REVIEW_FIRST_FLOOR {
        return Tier::ReviewFirst;
    }
    Tier::Advisory
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_identical_idents_is_type_one() {
        assert_eq!(classify(1.0, true), CloneType::Type1);
    }

    #[test]
    fn exact_renamed_idents_is_type_two() {
        assert_eq!(classify(1.0, false), CloneType::Type2);
    }

    #[test]
    fn near_miss_is_type_three() {
        assert_eq!(classify(0.9, true), CloneType::Type3);
    }

    #[test]
    fn tier_floors() {
        assert_eq!(tier_for(0.99, 0.8), Tier::AutoRefactor);
        assert_eq!(tier_for(0.9, 0.8), Tier::ReviewFirst);
        assert_eq!(tier_for(0.82, 0.8), Tier::Advisory);
    }
}
