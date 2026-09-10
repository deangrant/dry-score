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
  local report
  report="$(cargo run -q -p dry-rs -- . --format text --no-fail-on-findings)"
  echo "$report"
  if ! echo "$report" | grep -q 'findings=0'; then
    fail "dry-rs reported findings (expected findings=0)"
  fi
}

echo "verify: tier=$TIER (cwd=$ROOT)"
case "$TIER" in
  lite) run_lite ;;
  full) run_full ;;
esac

echo ""
echo "verify: OK ($TIER)"
