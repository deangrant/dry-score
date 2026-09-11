---
name: dry-rs-domain
description: >-
  Map dry-score crates, normalize/compare pipeline, clone tiers, walk/config
  knobs, and fingerprint stability. Use when changing dry-core, dry-rs emit,
  matching, CLI, report, or fixtures.
---

# dry-rs domain map

## Crates

| Crate | Role |
| --- | --- |
| [`crates/dry-core`](../../../crates/dry-core) | Language-agnostic domain, walk, config, compare, shared CLI/runner — **no** AST deps |
| [`crates/dry-rs`](../../../crates/dry-rs) | Rust `syn` normalizer implementing `LanguageNormalizer` |
| [`crates/dry-go`](../../../crates/dry-go) | Go Tree-sitter normalizer implementing `LanguageNormalizer` |

Language adapters belong in dedicated crates that reuse `dry-core` comparison.

## Pipeline

discover files → parse/normalize → fingerprint index → match → report

## Normalization invariants (Rust)

- Comments/whitespace discarded by the parser.
- Idents → positional placeholders; raw spellings kept in `ident_trace` for
  Type-1 vs Type-2.
- Literals → kind tags (`lit_int`, `lit_str`, …).
- Fingerprints must stay **toolchain-stable** and **location-independent**
  (do not bake spans or absolute paths into fingerprints).
- Emit helpers: recursive expr wrappers live under `normalize/emit/expr/`;
  [`shared.rs`](../../../crates/dry-rs/src/normalize/emit/shared.rs) stays pure
  (must not import `expr`).

## Matching

1. Exact buckets (identical fingerprint sets) → score `1.0`
2. Near-miss via inverted index + Jaccard; production vs test forms never pair
3. Sort most exact → least exact

## Labels

| Field | Values |
| --- | --- |
| `clone_type` | `type_1`, `type_2`, `type_3` |
| `tier` | `auto_refactor` (≥0.95), `review_first` (≥0.85), `advisory` (≥ threshold) |

## Config / walk

- Walk-up discovery loads [`dry.toml`](../../../dry.toml); schema in
  [`dry.example.toml`](../../../dry.example.toml).
- Key knobs: `gate.threshold`, `fail_on_findings`, `walk.min_nodes`,
  `walk.min_lines`, `walk.exclude`, `output.format`.
- Walker does **not** follow symlinks.

## Suppressions

Full-line `// dry-rs:ignore` (span) or `// dry-rs:ignore-file` (file). See
[`suppress.rs`](../../../crates/dry-rs/src/normalize/suppress.rs) and the
**dry-dogfood** skill for dogfood cleanup policy.

## Fixtures

Under [`crates/dry-rs/tests/fixtures/`](../../../crates/dry-rs/tests/fixtures/):
`type_1_exact`, `type_2_renamed`, `type_3_near_miss`, `non_clone`.
