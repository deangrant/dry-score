//! Smoke-test the `dry-rs` binary entrypoint for coverage of `main`.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dry-rs"))
}

#[test]
fn binary_help_and_analyze() {
    let help = bin().arg("--help").output();
    assert!(help.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let help = help.expect("run");
    assert!(help.status.success());
    let stdout = String::from_utf8_lossy(&help.stdout);
    assert!(stdout.contains("--threshold"));

    let fixture =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/non_clone");
    let run = bin()
        .arg(&fixture)
        .args([
            "--format",
            "text",
            "--no-fail-on-findings",
            "--min-nodes",
            "8",
        ])
        .output();
    assert!(run.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let run = run.expect("run");
    assert!(run.status.success());

    let json = bin()
        .arg(&fixture)
        .args([
            "--format",
            "json",
            "--no-fail-on-findings",
            "--min-nodes",
            "8",
        ])
        .output();
    assert!(json.is_ok());
}

#[test]
fn binary_unknown_flag_prints_usage_error() {
    let out = bin().arg("--not-a-real-flag").output();
    assert!(out.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let out = out.expect("run");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stderr.is_empty() || !stdout.is_empty(),
        "expected usage/error output"
    );
}
