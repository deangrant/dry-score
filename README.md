# dry-score

dry-score finds **structural clones** in Rust source. It compares normalized AST
forms—not raw text—scores pairs with Jaccard similarity over subtree
fingerprints, and routes findings into agentic tiers for CI and automation.

It is a duplication detector. It is not a style linter or a complexity scorer.

The shipped CLIs are **`dry-rs`** (Rust) and **`dry-go`** (Go).

## Requirements

- Rust toolchain **1.94.0** ([`rust-toolchain.toml`](rust-toolchain.toml))

## Build

```bash
cargo build --release -p dry-rs
```

Or run without installing a binary:

```bash
cargo run -p dry-rs -- path/to/crate
```

## Quick start

1. Build the release binary (see above).
2. Point `dry-rs` at one or more analysis roots (default: `.`).
3. Read the text report, or write JSON when you need machine output.

```bash
./target/release/dry-rs . --format both --json-out report.json
```

A text report lists clone type, score, tier, and member spans. With
`--fail-on-findings`, exit code `1` means the tool reported at least one
finding.

## Usage

```bash
dry-rs [PATH]... [options]
```

If you omit `PATH`, dry-rs analyzes `.`.

| Flag | Meaning |
| --- | --- |
| `--config PATH` | Load knobs from a TOML file |
| `--threshold FLOAT` | Minimum Jaccard score to report (default `0.85`; must be in `[0.0, 1.0]`) |
| `--format text\|json\|both` | Human summary, JSON envelope, or both (default `text`) |
| `--min-nodes N` | Drop forms smaller than N structural nodes (default `10`) |
| `--min-lines N` | Drop forms spanning fewer than N source lines (default `3`) |
| `--fail-on-findings` | Exit `1` when any finding is reported |
| `--no-fail-on-findings` | Do not fail the process on findings (overrides config) |
| `--json-out PATH` | When `--format both`, write JSON to this path |
| `--help`, `-h` | Print help and exit `0` |

Walk-up discovery loads `dry.toml` from the current directory or a parent unless
you pass `--config`. Analysis roots are trusted local trees; the walker does
**not** follow symlinks.

## Configuration

See [`dry.example.toml`](dry.example.toml) for the full annotated schema.
Defaults match the table below.

| Section | Key | Default |
| --- | --- | --- |
| `[gate]` | `threshold` | `0.85` |
| `[gate]` | `fail_on_findings` | `false` |
| `[output]` | `format` | `"text"` |
| `[walk]` | `extensions` | `["rs"]` |
| `[walk]` | `exclude` | `["target", ".git", "fixtures", "tests"]` |
| `[walk]` | `min_nodes` | `10` |
| `[walk]` | `min_lines` | `3` |

Setting `walk.exclude` in TOML **replaces** the default list. It does not merge
with the defaults.

## How detection works

Pipeline: discover files → parse/normalize → fingerprint → match → report.

### Normalize (Rust adapter)

- The parser discards comments and whitespace.
- Identifiers become positional placeholders so renamed twins share fingerprints.
- Raw identifier spellings stay in an `ident_trace` (occurrence sequence) for
  Type-1 versus Type-2.
- Literals become kind tags (`lit_int`, `lit_str`, …).
- Control flow, operators, and patterns keep structural labels.
- Macros fingerprint as name + delimiter + token-tree shape (not expansion).
- Each subtree hashes to a `u64` (fixed FNV-1a); the set of those hashes is the
  fingerprint.

### Match

1. Identical fingerprint sets score `1.0` (exact buckets).
2. Remaining forms use an inverted fingerprint index and greedy near-miss
   Jaccard (with a set-size window).
3. Production and test forms (`FormKind`) never pair.
4. Findings sort most exact → least exact.

### Labels

| Field | Values |
| --- | --- |
| `clone_type` | `type_1` (exact ids), `type_2` (renamed), `type_3` (near-miss) |
| `tier` | `auto_refactor` (≥0.95), `review_first` (≥0.85), `advisory` (≥ threshold and &lt; 0.85) |

When the configured threshold is ≥ 0.85, emitted findings do not use the
advisory band.

Deep module maps and invariants:
[`.agents/docs/ARCHITECTURE.md`](.agents/docs/ARCHITECTURE.md).

## Suppressions

Use a **full-line** `//` comment (optional leading whitespace). Trailing
comments and string substrings do not count.

- Span: `// dry-rs:ignore` or `// dry-rs:ignore. reason`
- File: `// dry-rs:ignore-file` near the top of the file

## Exit codes

| Code | Meaning | What to do |
| --- | --- | --- |
| `0` | Success (including `--help`) | Nothing required |
| `1` | Findings present and fail-on-findings is on | Inspect the report; fix clones or disable fail-on for report-only runs |
| `2` | Usage, config, analyze, or write error | Fix flags or `dry.toml`; check paths and permissions |

## Fixtures

Intentional corpora live under
[`crates/dry-rs/tests/fixtures/`](crates/dry-rs/tests/fixtures/):

| Corpus | Demonstrates |
| --- | --- |
| `type_1_exact` | Identical bodies and identifiers |
| `type_2_renamed` | Same structure, renamed locals/params |
| `type_3_near_miss` | Shared structure with a small edit |
| `non_clone` | Similar names, different control flow (no findings) |

## CI and local verify

| Workflow | Gate |
| --- | --- |
| [`lint.yml`](.github/workflows/lint.yml) | Workspace check, fmt, clippy `-D warnings`, rustdoc `-D warnings`, 500-line cap |
| [`test.yml`](.github/workflows/test.yml) | `cargo test --workspace --locked` |
| [`supply-chain.yml`](.github/workflows/supply-chain.yml) | `cargo deny` + `cargo audit` |
| [`dry-rs.yml`](.github/workflows/dry-rs.yml) | Release build; scan `.`; require **findings=0**; JSON artifact + step summary |

Local parity:

```bash
./scripts/verify.sh lite   # fmt, clippy, test
./scripts/verify.sh full   # lite + deny, audit, dry-rs findings=0
```

Contributor conventions: [AGENTS.md](AGENTS.md).

## Workspace layout

| Crate | Role |
| --- | --- |
| [`crates/dry-core`](crates/dry-core) | Language-agnostic domain, walk, config, compare, shared CLI/runner, reporters (no AST deps) |
| [`crates/dry-rs`](crates/dry-rs) | Rust `syn` adapter and the `dry-rs` binary |
| [`crates/dry-go`](crates/dry-go) | Go Tree-sitter adapter and the `dry-go` binary |

Language adapters implement `LanguageNormalizer` and reuse `dry-core`
comparison. See [ARCHITECTURE](.agents/docs/ARCHITECTURE.md) for the pipeline
and module map.

### Go (`dry-go`)

```bash
cargo build --release -p dry-go
./target/release/dry-go path/to/module
```

`dry-go` forces `walk.extensions` to `["go"]`. Suppress with full-line
`// dry-go:ignore` / `// dry-go:ignore-file`. Parsing uses Tree-sitter (C
grammar at build time); the adapter itself is Rust.

## License

[MIT](LICENSE) © 2026 Dean Grant
