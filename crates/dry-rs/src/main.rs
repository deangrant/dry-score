//! `dry-rs` — structural duplication detector for Rust sources.

use std::process::ExitCode;

use dry_rs::runner::{print_err, print_out, run_from_env};

fn main() -> ExitCode {
    match run_from_env() {
        Ok(code) => code,
        Err(err) => {
            if err.print_stdout {
                print_out(&err.message);
            } else {
                print_err(&err.message);
            }
            err.exit
        }
    }
}
