//! Positional identifier placeholders for rename-invariant matching.

use std::collections::HashMap;

/// Maps raw identifier spellings to stable placeholder labels.
#[derive(Debug, Default)]
pub struct PlaceholderMap {
    by_name: HashMap<String, u32>,
    next: u32,
    /// Ordered raw identifiers in first-seen order of occurrence.
    pub ident_trace: Vec<String>,
}

impl PlaceholderMap {
    /// Returns the placeholder label for `ident`, allocating if needed.
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
