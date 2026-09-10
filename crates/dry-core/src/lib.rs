//! Language-agnostic core for structural duplication analysis.
//!
//! Pipeline: discover → normalize (via [`LanguageNormalizer`]) → compare →
//! report. Adapters supply parsed forms; this crate never depends on an AST
//! library.

#![forbid(unsafe_code)]

pub mod analyze;
pub mod compare;
pub mod config;
pub mod domain;
pub mod ports;
pub mod report;
pub mod walk;

#[doc(inline)]
pub use analyze::{AnalysisResult, analyze};
#[doc(inline)]
pub use compare::compare;
#[doc(inline)]
pub use config::{Config, OutputFormat, discover_config, load_config};
#[doc(inline)]
pub use domain::{
    CloneType, Finding, FormKind, FormMember, FormSpan, NormalizedForm, ReportSummary, Tier,
};
#[doc(inline)]
pub use ports::{LanguageNormalizer, NormalizeError, NormalizeOutcome};
#[doc(inline)]
pub use report::{Report, render_json, render_text};
#[doc(inline)]
pub use walk::{WalkError, WalkOptions, collect_source_files};
