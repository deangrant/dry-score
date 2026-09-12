---
name: verify-gates
description: >-
  Run dry-score quality gates (lite or full verify). Use at the end of
  implementation plans, before claiming green, or when the user asks to verify,
  run the pipeline, or dogfood gates.
---

# Verify gates

Canonical quality gates for this repository. Prefer
[`scripts/verify.sh`](../../../scripts/verify.sh) over ad-hoc command lists.

## Tiers

| Tier | Steps |
| --- | --- |
| `lite` | `cargo fmt --all` → clippy `-D warnings` → `cargo test --workspace` |
| `full` | `lite` + `cargo deny check` + `cargo audit` + dry-rs self-scan (**findings=0**) + dry-go dogfood scan (**findings=0**) |

Default when finishing substantial work or “implement the plan”: **`full`**.

```bash
./scripts/verify.sh lite
./scripts/verify.sh full   # or: ./scripts/verify.sh
```

## Hard prohibitions

- Do **not** relax [`clippy.toml`](../../../clippy.toml) thresholds to make
  gates pass.
- Do **not** add or edit [`dry.toml`](../../../dry.toml) solely to hide
  self-scan findings.
- Do **not** claim green without running the appropriate tier.

## Success criteria

- `lite`: fmt/clippy/test exit 0
- `full`: above, plus deny/audit exit 0, dry-rs report contains `findings=0`,
  dry-go dogfood report contains `findings=0`
