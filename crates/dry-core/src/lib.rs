//! Language-agnostic core for structural duplication analysis.
//!
//! Pipeline: discover → normalize (via [`LanguageNormalizer`]) → compare →
//! report. Adapters supply parsed forms; this crate never depends on an AST
//! library.

#![forbid(unsafe_code)]

pub mod analyze;
pub mod cli;
pub mod compare;
pub mod config;
pub mod domain;
pub mod norm;
pub mod placeholders;
pub mod ports;
pub mod report;
pub mod runner;
pub mod suppress;
pub mod walk;

#[doc(inline)]
pub use analyze::{AnalysisResult, analyze};
#[doc(inline)]
pub use cli::{CliArgs, CliError, CliOptions, help_text, parse_args};
#[doc(inline)]
pub use compare::compare;
#[doc(inline)]
pub use config::{Config, OutputFormat, discover_config, load_config, validate_threshold};
#[doc(inline)]
pub use domain::{
    CloneType, Finding, FormKind, FormMember, FormSpan, NormalizedForm, ReportSummary, Tier,
};
#[doc(inline)]
pub use norm::{FingerprintResult, NormNode, fingerprint_tree};
#[doc(inline)]
pub use placeholders::PlaceholderMap;
#[doc(inline)]
pub use ports::{LanguageNormalizer, NormalizeError, NormalizeOutcome};
#[doc(inline)]
pub use report::{Report, render_json, render_text};
#[doc(inline)]
pub use runner::{
    below_size_thresholds, emit_report, exit_for_findings, exit_from_cli_result, print_err,
    print_out, run_analysis,
};
#[doc(inline)]
pub use suppress::{file_is_ignored, span_is_ignored};
#[doc(inline)]
pub use walk::{WalkError, WalkOptions, collect_source_files};
