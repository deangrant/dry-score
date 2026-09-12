//! `dry-rs` — structural duplication detector for Rust sources.

use std::process::ExitCode;

use dry_core::exit_from_cli_result;
use dry_rs::runner::run_from_env;

fn main() -> ExitCode {
    exit_from_cli_result(run_from_env())
}
