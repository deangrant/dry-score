# dry-score

dry-score finds **structural clones** in Rust source. It compares normalized AST
forms—not raw text—scores pairs with multiset Jaccard similarity over subtree
fingerprint bags, and routes findings into agentic tiers for CI and automation.

It is a duplication detector. It is not a style linter or a complexity scorer.

The shipped CLIs are **`dry-rs`** (Rust), **`dry-go`** (Go), **`dry-ts`**
(TypeScript), and **`dry-py`** (Python).

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
| `--min-nodes N` | Drop forms smaller than N structural nodes (default `10`; must be `>= 1`) |
| `--min-lines N` | Drop forms spanning fewer than N source lines (default `3`; must be `>= 1`) |
| `--max-file-bytes N` | Skip source files larger than N bytes (default `2097152`; must be `>= 1`) |
| `--extensions EXT[,EXT]...` | Replace `walk.extensions` (comma-separated, no dots; unsupported on language-forced adapters) |
| `--exclude NAME[,NAME]...` | Merge names into `walk.exclude` (keeps built-in defaults) |
| `--exclude-only NAME[,NAME]...` | Replace `walk.exclude` entirely (drops built-in defaults) |
| `--fail-on-findings` | Exit `1` when any finding is reported |
| `--no-fail-on-findings` | Do not fail the process on findings (overrides config) |
| `--json-out PATH` | Required when format is `both`: write JSON here (overwrites if present) |
| `--help`, `-h` | Print help and exit `0` |

Walk-up discovery loads `dry.toml` from the current directory or a parent,
then from the first analysis path if still missing, unless you pass `--config`.
Analysis roots are trusted local trees; the walker does **not** follow
symlinks, and a symlink root is an error. Concurrent path replacement during a
scan is outside the trust model. `--json-out` may target any writable path and
overwrites existing files.

## Configuration

See [`dry.example.toml`](dry.example.toml) for the full annotated schema.
Defaults match the table below.

| Section | Key | Default |
| --- | --- | --- |
| `[gate]` | `threshold` | `0.85` |
| `[gate]` | `fail_on_findings` | `false` |
| `[output]` | `format` | `"text"` |
| `[walk]` | `extensions` | `["rs"]` |
| `[walk]` | `exclude` | `["target", ".git", "fixtures", "node_modules", "vendor", ".venv", "venv", "dist", "__pycache__"]` |
| `[walk]` | `exclude_replace` | `false` |
| `[walk]` | `min_nodes` | `10` |
| `[walk]` | `min_lines` | `3` |
| `[walk]` | `max_file_bytes` | `2097152` (2 MiB) |

Setting `walk.exclude` in TOML **merges** with the built-in defaults (deduped).
Defaults skip common dependency and build dirs across Rust, Node, Go, and
Python. Add `"tests"` to skip integration-test trees. Set
`walk.exclude_replace = true` (or pass `--exclude-only`) to use only the listed
names.

## How detection works

Pipeline: discover files → parse/normalize → fingerprint → match → report.

### Normalize (Rust adapter)

- The parser discards comments and whitespace.
- Identifiers become positional placeholders so renamed twins share fingerprints.
- Raw identifier spellings stay in an `ident_trace` (occurrence sequence) for
  Type-1 versus Type-2.
- Literals become kind tags (`lit_int`, `lit_str`, …).
- Control flow, operators, and patterns keep structural labels.
- Macros: an allowlist (`vec`, `assert*`, `format`, `print*`/`eprint*`, `dbg`,
  `matches`, …) expands to normalized expression children when the path is bare
  or rooted at `std`/`core`/`alloc`; other macros (including custom
  `crate::assert_eq!`) keep name + delimiter + token-tree shape.
- Closures are extracted as named forms (`{parent}.$closure:L{line}`), in
  addition to remaining embedded in the parent body.
- Each subtree hashes to a `u64` (fixed FNV-1a); the bag of those hashes
  (hash → multiplicity) is the fingerprint.

### Match

1. Identical fingerprint bags score `1.0` (exact buckets).
2. Remaining forms use an inverted fingerprint index and connected-component
   near-miss multiset Jaccard (window on total bag size; DF-ordered occurrence
   prefix for candidates; score is the minimum pairwise Jaccard among members;
   non-threshold-closed components split into exclusive pairs).
3. Production and test forms (`FormKind`) never pair. Kind is
   language-idiomatic: Rust attrs/`cfg(test)`, Go `*_test.go` only, TypeScript
   `.test.` / `.spec.` / `__tests__`, Python `test_*.py` / `*_test.py` /
   `tests` / `test` path components. Importable harness packages outside those
   conventions stay `Production` so prod clones remain visible.
4. Findings sort most exact → least exact.

### Labels

| Field | Values |
| --- | --- |
| `clone_type` | `type_1` (exact ids), `type_2` (renamed), `type_3` (near-miss) |
| `tier` | `auto_refactor` (≥0.95), `review_first` (≥0.85), `advisory` (≥ threshold and &lt; 0.85) |

When the configured threshold is ≥ 0.85, emitted findings do not use the
advisory band.

Text report banners use `scanned=` for `summary.files_scanned` (files that
returned `Ok` from normalize). Size skips, I/O failures, and parse errors are
not counted there; they appear under `warnings=` / the warnings section. JSON
still uses the field name `files_scanned`.

Deep module maps and invariants:
[`.agents/docs/ARCHITECTURE.md`](.agents/docs/ARCHITECTURE.md).

## Suppressions

Use a **full-line** comment directive (optional leading whitespace): `//`,
`///`, `//!`, or a whole-line `/* … */`. Trailing comments and string
substrings do not count.

- Span: `// dry-rs:ignore` or `// dry-rs:ignore. reason`
- File: `// dry-rs:ignore-file` near the top of the file
- Doc/block forms (`///`, `//!`, `/* … */`) are also recognized

## Exit codes

| Code | Meaning | What to do |
| --- | --- | --- |
| `0` | Success (including `--help`) | Nothing required |
| `1` | Findings present and fail-on-findings is on | Inspect the report; fix clones or disable fail-on for report-only runs |
| `2` | Usage, config, analyze, write, or JSON serialize error | Fix flags or `dry.toml`; check paths and permissions. JSON emit fails closed (no alternate error schema). |

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
./scripts/verify.sh full   # lite + deny, audit, dry-rs + dry-go + dry-ts + dry-py dogfood findings=0
```

Contributor conventions: [AGENTS.md](AGENTS.md).

## Workspace layout

| Crate | Role |
| --- | --- |
| [`crates/dry-core`](crates/dry-core) | Language-agnostic domain, walk, config, compare, shared CLI/runner, reporters (no AST deps) |
| [`crates/dry-rs`](crates/dry-rs) | Rust `syn` adapter and the `dry-rs` binary |
| [`crates/dry-go`](crates/dry-go) | Go Tree-sitter adapter and the `dry-go` binary |
| [`crates/dry-ts`](crates/dry-ts) | TypeScript Tree-sitter adapter and the `dry-ts` binary |
| [`crates/dry-py`](crates/dry-py) | Python Tree-sitter adapter and the `dry-py` binary |

Language adapters implement `LanguageNormalizer` and reuse `dry-core`
comparison. See [ARCHITECTURE](.agents/docs/ARCHITECTURE.md) for the pipeline
and module map.

### Go (`dry-go`)

```bash
cargo build --release -p dry-go
./target/release/dry-go path/to/module
```

`dry-go` forces `walk.extensions` to `["go"]` (CLI `--extensions` is rejected).
Suppress with full-line `// dry-go:ignore` / `// dry-go:ignore-file`. Parsing uses
Tree-sitter (C grammar at build time); syntax errors discard the file’s forms
and surface as analyze warnings (same fail-closed contract as `dry-rs`). Full
verify and CI dogfood scan [`crates/dry-go/dogfood/`](crates/dry-go/dogfood/)
(tiny unique non-clone smoke tree plus exclude decoys under `vendor/`; clone
types are covered by fixtures, not dogfood breadth). The adapter itself is
Rust.

### TypeScript (`dry-ts`)

```bash
cargo build --release -p dry-ts
./target/release/dry-ts path/to/project
```

`dry-ts` forces `walk.extensions` to `["ts", "tsx", "mts", "cts"]` (not `.js` /
`.jsx`; CLI `--extensions` is rejected). Declaration files (`*.d.ts` / `*.d.mts` /
`*.d.cts`) are skipped. Suppress with full-line `// dry-ts:ignore` /
`// dry-ts:ignore-file`. Parsing uses Tree-sitter TypeScript / TSX grammars;
syntax errors discard the file’s forms and surface as analyze warnings (same
fail-closed contract as `dry-rs`). Full verify and CI dogfood scan
[`crates/dry-ts/dogfood/`](crates/dry-ts/dogfood/) (tiny unique non-clone smoke
tree plus exclude decoys under `node_modules/`; clone types are covered by
fixtures, not dogfood breadth). The adapter itself is Rust.

### Python (`dry-py`)

```bash
cargo build --release -p dry-py
./target/release/dry-py path/to/package
```

`dry-py` forces `walk.extensions` to `["py"]` (CLI `--extensions` is rejected).
Stub files (`*.pyi`) are skipped. Suppress with full-line `# dry-py:ignore` /
`# dry-py:ignore-file`. Parsing uses Tree-sitter Python; syntax errors discard
the file’s forms and surface as analyze warnings (same fail-closed contract as
`dry-rs`). Full verify and CI dogfood scan
[`crates/dry-py/dogfood/`](crates/dry-py/dogfood/) (tiny unique non-clone smoke
tree plus exclude decoys under `.venv/`; clone types are covered by fixtures,
not dogfood breadth). The adapter itself is Rust.

## License

[MIT](LICENSE) © 2026 Dean Grant
