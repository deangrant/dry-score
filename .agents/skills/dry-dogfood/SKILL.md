---
name: dry-dogfood
description: >-
  Clear dry-rs self-scan findings on this repo (dedupe, ignore intentional
  parallels, merge clone-y tests). Use after CC splits, before merge, or when
  dry-rs reports findings on `.` / dogfood cleanup.
---

# dry-rs dogfood cleanup

Drive self-scan findings to **zero** without undoing CC≤5 splits or relaxing
gates.

## Procedure

1. Scan (respects walk-up [`dry.toml`](../../../dry.toml)):

   ```bash
   cargo run -p dry-rs -- . --format text --no-fail-on-findings
   ```

2. For each finding cluster, classify:

   | Kind | Action |
   | --- | --- |
   | Same rule, duplicated helpers | **Dedupe** into one shared helper |
   | Intentional parallel shapes (CC-split match arms, one-flag CLI appliers, `bin_*_label` families, `try_emit_*` shells) | **`// dry-rs:ignore. <reason>`** full-line comment inside the form span |
   | Near-identical tests | **Merge** into one corpus test, or ignore with reason |

3. **Never** re-merge helpers that were split only to keep cognitive complexity
   ≤5 / clippy thresholds.

4. Re-scan until the report shows `findings=0`.

5. Confirm CRAP still green via `./scripts/verify.sh full` (or at least the
   llvm-cov + crap steps). Success = **findings=0 and crap still green**.

## Ignore marker format

Markers must be a **full-line** `//` comment (optional leading whitespace).
Trailing comments and string substrings do not count. See
[`suppress.rs`](../../../crates/dry-rs/src/normalize/suppress.rs).

```rust
fn apply_threshold(...) -> Result<(), CliError> {
    // dry-rs:ignore. CC-driven one-flag CLI helpers; parallel shape is intentional.
    ...
}
```

- Span: `// dry-rs:ignore` or `// dry-rs:ignore. reason`
- File: `// dry-rs:ignore-file` near the top of the file

## Prohibitions

- Do not lower `--threshold` / `min_nodes` / `min_lines` to hide clones.
- Do not edit [`dry.toml`](../../../dry.toml) solely to exclude production paths
  that should stay clean.
