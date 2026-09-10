---
name: verify-gates
description: >-
  Run dry-score quality gates (lite or full verify). Use at the end of
  implementation plans, before claiming green, or when the user asks to verify,
  run the pipeline, dogfood gates, llvm-cov, or crap threshold checks.
---

# Verify gates

Canonical quality gates for this repository. Prefer
[`scripts/verify.sh`](../../../scripts/verify.sh) over ad-hoc command lists.

## Tiers

| Tier | Steps |
| --- | --- |
| `lite` | `cargo fmt --all` → clippy `-D warnings` → `cargo test --workspace` |
| `full` | `lite` + `cargo deny check` + `cargo audit` + dry-rs self-scan (**findings=0**) + llvm-cov → `lcov.info` + crap-rs `--fail-above --threshold 5` |

Default when finishing substantial work or “implement the plan”: **`full`**.

```bash
./scripts/verify.sh lite
./scripts/verify.sh full   # or: ./scripts/verify.sh
```

## Crap binary

`full` runs crap via:

```bash
cargo run --manifest-path "$CRAP_MANIFEST" -p crap-rs --locked -- --fail-above --threshold 5
```

Default `CRAP_MANIFEST=/home/deangrant/github/rust-crap/Cargo.toml`. Override with
env if the checkout lives elsewhere.

## Hard prohibitions

- Do **not** relax [`clippy.toml`](../../../clippy.toml) thresholds or CRAP
  `--threshold` to make gates pass.
- Do **not** add or edit [`dry.toml`](../../../dry.toml) solely to hide
  self-scan findings.
- Do **not** claim green without running the appropriate tier.

## Success criteria

- `lite`: fmt/clippy/test exit 0
- `full`: above, plus deny/audit exit 0, dry-rs report contains `findings=0`,
  crap reports no functions exceeding threshold 5
