//! Smoke tests for the `dry-go` binary.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dry-go"))
}

#[test]
fn binary_help_and_analyze() {
    let help = bin().arg("--help").output();
    assert!(help.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let help = help.expect("run");
    assert!(help.status.success());
    let stdout = String::from_utf8_lossy(&help.stdout);
    assert!(stdout.starts_with("dry-go [PATH]..."));

    let fixture =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/non_clone");
    let run = bin().arg(&fixture).args(["--format", "text", "--no-fail-on-findings"]).output();
    assert!(run.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let run = run.expect("run");
    assert!(run.status.success());
    let text = String::from_utf8_lossy(&run.stdout);
    assert!(text.contains("findings="));
}

#[test]
fn binary_unknown_flag_prints_usage_error() {
    let out = bin().arg("--not-a-flag").output();
    assert!(out.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let out = out.expect("run");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("unknown flag"));
}
