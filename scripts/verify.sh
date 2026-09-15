#!/usr/bin/env bash
# Workspace quality gates for dry-score.
# Usage: scripts/verify.sh [lite|full]
# Default: full
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TIER="${1:-full}"
case "$TIER" in
  lite|full) ;;
  *)
    echo "usage: scripts/verify.sh [lite|full]" >&2
    exit 2
    ;;
esac

step() {
  echo ""
  echo "==> $*"
}

fail() {
  echo "verify: FAIL: $*" >&2
  exit 1
}

# Runs a dogfood scan, tees the text report, and requires findings=0.
# Args: failure label, then the cargo/command argv.
require_findings_zero() {
  local label="$1"
  shift
  local tmp
  tmp="$(mktemp)"
  "$@" | tee "$tmp"
  if ! grep -q 'findings=0' "$tmp"; then
    rm -f "$tmp"
    fail "${label} reported findings (expected findings=0)"
  fi
  rm -f "$tmp"
}

run_lite() {
  step "cargo fmt --all"
  cargo fmt --all

  step "cargo clippy (deny warnings)"
  cargo clippy --workspace --all-targets --all-features -- -D warnings

  step "cargo test --workspace"
  cargo test --workspace
}

run_full() {
  run_lite

  step "cargo deny check"
  cargo deny check

  step "cargo audit"
  cargo audit

  step "dry-rs self-scan (require findings=0)"
  require_findings_zero "dry-rs" \
    cargo run -q -p dry-rs -- . --format text --no-fail-on-findings

  step "dry-go dogfood scan (require findings=0)"
  require_findings_zero "dry-go" \
    cargo run -q -p dry-go -- crates/dry-go/dogfood --format text --no-fail-on-findings

  step "dry-ts dogfood scan (require findings=0)"
  require_findings_zero "dry-ts" \
    cargo run -q -p dry-ts -- crates/dry-ts/dogfood --format text --no-fail-on-findings

  step "dry-py dogfood scan (require findings=0)"
  require_findings_zero "dry-py" \
    cargo run -q -p dry-py -- crates/dry-py/dogfood --format text --no-fail-on-findings
}

echo "verify: tier=$TIER (cwd=$ROOT)"
case "$TIER" in
  lite) run_lite ;;
  full) run_full ;;
esac

echo ""
echo "verify: OK ($TIER)"
