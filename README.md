# dry-rs

Structural duplication detector for Rust. It finds Type-1 / Type-2 / Type-3
clones by comparing **normalized AST structure** (not raw text), scores pairs
with Jaccard similarity over subtree fingerprints, and routes findings into
agentic tiers for CI and automation.

## Quick start

```bash
cargo build --release -p dry-rs
./target/release/dry-rs path/to/crate --format both --json-out report.json
```

Options:

| Flag | Meaning |
| --- | --- |
| `--threshold FLOAT` | Minimum Jaccard score to report (default `0.85`) |
| `--format text\|json\|both` | Human summary, JSON envelope, or both |
| `--config PATH` | Load knobs from a TOML file |
| `--min-nodes N` | Drop forms smaller than N structural nodes |
| `--fail-on-findings` | Exit `1` when any finding is reported |
| `--json-out PATH` | When `--format both`, write JSON to this path |

Walk-up discovery loads `dry.toml` from the current directory or a parent.
See [`dry.example.toml`](dry.example.toml) for the full schema.

Exit codes: `0` success, `1` findings (only with fail-on), `2` usage/config error.

## Architecture

| Crate | Role |
| --- | --- |
| [`crates/dry-core`](crates/dry-core) | Language-agnostic domain, comparison, walk, config, reporters |
| [`crates/dry-rs`](crates/dry-rs) | CLI + Rust `syn` normalizer |

`dry-core` has **no AST dependencies**. Future language adapters implement
`LanguageNormalizer` and reuse the same comparison engine.

Pipeline: discover files → parse/normalize → fingerprint index → match → report.

## Detection semantics

**Normalization (Rust adapter):**

- Comments and whitespace are discarded by the parser.
- Identifiers become positional placeholders so renamed twins share fingerprints.
- Raw identifier spellings are retained in an `ident_trace` for Type-1 vs Type-2.
- Literals become kind tags (`lit_int`, `lit_str`, …).
- Control-flow and operators keep structural labels.
- Each subtree hashes to a `u64`; the set of subtree hashes is the fingerprint.

**Matching:**

1. Exact buckets: identical fingerprint sets → score `1.0`
2. Near-miss: sliding-window Jaccard with size-ratio early break
3. Sort findings most exact → least exact

**Labels:**

| Field | Values |
| --- | --- |
| `clone_type` | `type_1` (exact ids), `type_2` (renamed), `type_3` (near-miss) |
| `tier` | `auto_refactor` (≥0.95), `review_first` (≥0.85), `advisory` (≥ threshold) |

Suppress a span with `// dry-rs:ignore`, or a whole file with
`// dry-rs:ignore-file` near the top.

## Fixtures

Intentional corpora live under
[`crates/dry-rs/tests/fixtures/`](crates/dry-rs/tests/fixtures/):

- `type_1_exact` — identical bodies and identifiers
- `type_2_renamed` — identical structure, renamed locals/params
- `type_3_near_miss` — shared structure with a small edit
- `non_clone` — similar names, different control flow

## CI

[`.github/workflows/dry-rs.yml`](.github/workflows/dry-rs.yml) builds the
release binary on every PR/push, analyzes library sources, uploads JSON, and
writes a text step summary (`fail-on-findings` off until a clean baseline).

## Workspace tooling

This repo also carries opinionated lint CI, supply-chain checks, and agent
guidance:

| Area | Location |
| --- | --- |
| Workspace + lints | [`Cargo.toml`](Cargo.toml) |
| Clippy thresholds | [`clippy.toml`](clippy.toml) (cognitive 8, type 200, fn 50 lines) |
| Toolchain | [`rust-toolchain.toml`](rust-toolchain.toml) (**1.94.0**) |
| Lint CI | [`.github/workflows/lint.yml`](.github/workflows/lint.yml) |
| Supply chain | [`.github/workflows/supply-chain.yml`](.github/workflows/supply-chain.yml) |
| Agent index | [AGENTS.md](AGENTS.md) |

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
cargo audit
cargo test --workspace
```

## License

[MIT](LICENSE) © 2026 Dean Grant
