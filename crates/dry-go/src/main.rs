//! `dry-go` — structural duplication detector for Go sources.

use std::process::ExitCode;

use dry_core::exit_from_cli_result;
use dry_go::runner::run_from_env;

fn main() -> ExitCode {
    exit_from_cli_result(run_from_env())
}
