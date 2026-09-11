//! Lightweight normalized tree nodes.

/// One node in the language-agnostic structural tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormNode {
    /// Structural label (operator, control-flow, placeholder, …).
    pub label: String,
    /// Ordered children.
    pub children: Vec<Self>,
}

impl NormNode {
    /// Leaf node with no children.
    #[must_use]
    pub fn leaf(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            children: Vec::new(),
        }
    }

    /// Interior node with children.
    #[must_use]
    pub fn branch(label: impl Into<String>, children: Vec<Self>) -> Self {
        Self {
            label: label.into(),
            children,
        }
    }
}
