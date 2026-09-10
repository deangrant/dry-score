//! Domain types for forms, findings, and report summaries.

mod enums;
mod finding;
mod form;
mod span;
mod summary;

#[doc(inline)]
pub use enums::{CloneType, FormKind, Tier};
#[doc(inline)]
pub use finding::{Finding, FormMember};
#[doc(inline)]
pub use form::NormalizedForm;
#[doc(inline)]
pub use span::FormSpan;
#[doc(inline)]
pub use summary::ReportSummary;
