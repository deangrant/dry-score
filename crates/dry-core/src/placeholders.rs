//! Positional identifier placeholders for rename-invariant matching.

use std::collections::HashMap;

/// Maps raw identifier spellings to stable placeholder labels.
#[derive(Debug, Default)]
pub struct PlaceholderMap {
    by_name: HashMap<String, u32>,
    next: u32,
    /// Raw identifier spellings in [`Self::placeholder`] call order; repeats included.
    ///
    /// Distinct from first-seen allocation of `idN` labels in `by_name`.
    pub ident_trace: Vec<String>,
}

impl PlaceholderMap {
    /// Returns the placeholder label for `ident`, allocating if needed.
    ///
    /// Each call appends the raw spelling to [`Self::ident_trace`].
    pub fn placeholder(&mut self, ident: &str) -> String {
        self.ident_trace.push(ident.to_owned());
        let idx = if let Some(existing) = self.by_name.get(ident) {
            *existing
        } else {
            let idx = self.next;
            self.next = self.next.saturating_add(1);
            self.by_name.insert(ident.to_owned(), idx);
            idx
        };
        format!("id{idx}")
    }
}
