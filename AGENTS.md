# Contributor Guidance

Guidance for AI agents and humans working in this repository.

**New task?** Glance → route → skill → change → verify.

This file is the entry point for repository conventions. Keep detailed architecture,
implementation guidance, and task-specific instructions in the referenced files rather
than duplicating them here.

Canonical agent assets live under [`.agents/`](.agents/); [`.cursor/rules`](.cursor/rules),
[`.cursor/commands`](.cursor/commands), and [`.cursor/hooks.json`](.cursor/hooks.json)
symlink or point there for editor integration.

## Repository at a glance

dry-score is a virtual Cargo workspace that detects **structural clones** by comparing
normalized AST forms (not raw text), scoring pairs with Jaccard similarity over subtree
fingerprints, and routing findings into agentic tiers. It is a duplication detector, not
a style linter or complexity scorer.

| Package | Path | Role |
| ------- | ---- | ---- |
| Core | `crates/dry-core` | Domain, walk, config, analyze, compare, reporters (no AST deps) |
| Rust adapter | `crates/dry-rs` | CLI, `syn` normalizer, and the `dry-rs` binary |

**Hard invariants (never violate):**

- `dry-core` has no AST dependencies. Adapters own parsing; scoring stays in `dry-core`.
- Adapters implement `LanguageNormalizer`. Do not fork Jaccard or tier rules in an adapter.
- Fingerprints are toolchain-stable and location-independent (no spans or absolute paths in hashes).
- `emit/shared` must not import `expr` (recursive wraps live in `expr/wrap`).
- The walker does not follow symlinks.
- Production and test forms (`FormKind`) never pair.
- Workspace members are `dry-core` and `dry-rs` only.
- No `#[allow]`. Suppressions must be `#[expect(..., reason = "...")]`.

**Runtime:** Rust toolchain `1.94.0`. Lean loop: `./scripts/verify.sh lite`. Full local
gates: `./scripts/verify.sh full` or `/verify` (see
[verify-gates](.agents/skills/verify-gates/SKILL.md)).

## Instruction Precedence

When instructions conflict, apply the most specific applicable instruction:

1. Repository-level `AGENTS.md`
2. Applicable files under `.agents/rules/`
3. Applicable files under `.agents/skills/`
4. Relevant documentation under `.agents/docs/`
5. Existing local implementation conventions

More specific guidance takes precedence over general guidance.

Rules define repository constraints and invariants. Skills provide task-specific
implementation guidance. Documentation provides architectural and product context.

[`gate-contract`](.agents/rules/gate-contract/) is `alwaysApply: true` and reminds agents
to run the correct verify tier. Other rules are opt-in or glob-scoped. This file tells you
**when to load** skills and docs; rules state **what you must not do**. If a skill suggests
something a rule forbids, the rule wins.

## Operating Principles

- Make the smallest change that correctly solves the task.
- Preserve existing crate boundaries and public contracts unless the task requires changing them.
- Prefer existing workspace crates, utilities, and patterns before introducing new abstractions or dependencies.
- Do not refactor unrelated code while completing a task.
- Do not weaken, bypass, or remove repository rules to make a change easier.
- Keep changes focused, reviewable, and consistent with the surrounding code.
- Prefer concrete diffs over speculative refactors. Match existing style; skip filler.
- Treat configuration (`dry.toml`), public CLI flags, and report contracts as significant; inspect their conventions before modifying them.
- Keep functions under [`clippy.toml`](clippy.toml) thresholds instead of silencing complexity.
- Keep `.rs` files at or under 500 lines.

## Common mistakes

- Forking Jaccard similarity or tier floors into the Rust adapter instead of `dry-core`.
- Baking spans or absolute paths into fingerprints (breaks toolchain-stable matching).
- Letting `normalize/emit/shared` import `expr` (reintroduces a module cycle).
- Following symlinks in the walker or assuming out-of-root links are analyzed.
- Re-merging CC≤5 dispatch shells that were split for Clippy, then fighting dry-rs dogfood.
- Relaxing Clippy thresholds or editing `dry.toml` solely to hide self-scan findings.
- Claiming `/verify` or CI passed without running the commands.

## Workflow

Before changing code:

1. Inspect the repository status and relevant diff.
2. Identify the crate(s), files, and architectural area affected.
3. Read the applicable rules.
4. Read the matching skill(s) before modifying that area.
5. Check [ARCHITECTURE.md](.agents/docs/ARCHITECTURE.md) when the change crosses crate or pipeline boundaries.
6. Make the smallest appropriate change.
7. Run the narrowest relevant tests and checks first.
8. Run `/verify` (or `./scripts/verify.sh`) before considering the change complete when practical.
   Procedure: [verify-gates](.agents/skills/verify-gates/SKILL.md).
9. Review the final diff for unintended changes.

Do not read every skill or document by default. Load only the guidance relevant to
the task being performed.

## Task routing

Read the matching skill **before** editing that area. Load only what the task needs.

| If you are changing… | Read first |
| -------------------- | ---------- |
| Normalize, compare, tiers, walk/config map | [`dry-rs-domain`](.agents/skills/dry-rs-domain/) |
| Self-scan findings, `// dry-rs:ignore`, dogfood cleanup | [`dry-dogfood`](.agents/skills/dry-dogfood/) |
| Local verify tiers, gate prohibitions | [`verify-gates`](.agents/skills/verify-gates/) |
| Rust style, docs, naming, API conventions | [`rust-style-guide`](.agents/skills/rust-style-guide/) |
| Traits, modules, dependency direction | [`rust-solid-design`](.agents/skills/rust-solid-design/) |

Cross-crate or pipeline changes: read
[ARCHITECTURE.md](.agents/docs/ARCHITECTURE.md) and every affected skill.

## Repository Documentation

- [README.md](README.md) — product overview, detection semantics, fixtures, CI notes
- [`.agents/docs/ARCHITECTURE.md`](.agents/docs/ARCHITECTURE.md) — crate boundaries, analysis pipeline, module maps, and invariants
- [`Cargo.toml`](Cargo.toml) — virtual workspace members and maximum `[workspace.lints]`
- [`clippy.toml`](clippy.toml) — complexity and line thresholds
- [`rustfmt.toml`](rustfmt.toml) — `max_width` 100
- [`deny.toml`](deny.toml) — cargo-deny policy
- [`rust-toolchain.toml`](rust-toolchain.toml) — pinned toolchain and components
- [`dry.toml`](dry.toml) / [`dry.example.toml`](dry.example.toml) — dogfood config and schema
- [`.github/workflows/`](.github/workflows/) — lint, supply-chain, dry-rs CI

## Rules

Canonical repository rules live under [`.agents/rules/`](.agents/rules/). The directory
is symlinked from [`.cursor/rules`](.cursor/rules).

### Always-on

- [`.agents/rules/gate-contract/`](.agents/rules/gate-contract/) — run `verify` lite/full after Rust edits; never relax thresholds to hide failures

### Opt-in

- [`.agents/rules/ai-slop-mitigation/`](.agents/rules/ai-slop-mitigation/) — concrete diffs, no filler

## Skills

Canonical skills live under [`.agents/skills/`](.agents/skills/).

Read the matching skill before changing the corresponding area. Skills are
task-specific guidance and should not be loaded unless relevant.

- [`.agents/skills/rust-style-guide/`](.agents/skills/rust-style-guide/) — formatting, docs, naming, API conventions
- [`.agents/skills/rust-solid-design/`](.agents/skills/rust-solid-design/) — SOLID in Rust: traits, modules, DI
- [`.agents/skills/verify-gates/`](.agents/skills/verify-gates/) — lite/full verify tiers and prohibitions
- [`.agents/skills/dry-dogfood/`](.agents/skills/dry-dogfood/) — self-scan cleanup (dedupe vs ignore)
- [`.agents/skills/dry-rs-domain/`](.agents/skills/dry-rs-domain/) — crates, tiers, normalize/compare map

## Commands

Canonical slash commands live under [`.agents/commands/`](.agents/commands/).
The directory is symlinked from [`.cursor/commands`](.cursor/commands).

Prefer repository commands over manually recreating equivalent workflows.

- `/verify` — `./scripts/verify.sh` lite or full (fmt, Clippy, test; full adds deny, audit, dry-rs findings=0)
- `/dry-dogfood` — clear dry-rs self-scan findings to zero without relaxing gates
- `/design-scan` — style + SOLID checklist with must-fix / nice-to-have / keep-as-is

## Hooks

Hook configuration lives in [`.cursor/hooks.json`](.cursor/hooks.json).

Hooks may auto-format after edit. They do not guarantee correctness. Always run
`/verify` explicitly before claiming done.

- `afterFileEdit` → [`.agents/hooks/rustfmt.sh`](.agents/hooks/rustfmt.sh) — formats edited `*.rs` with rustfmt (fail-open)

## Definition of done

A change is complete when:

1. Only intended files changed (review `git diff`).
2. Applicable rules and skills were followed for touched crates.
3. Narrow tests for touched paths passed (see table below).
4. `/verify` was run when the change is merge-ready, or you explicitly report what was skipped and why.
5. README or ARCHITECTURE were updated if behavior or crate contracts changed.

| Touched area | Minimum verification |
| ------------ | -------------------- |
| Any `.rs` / workspace code | `./scripts/verify.sh lite` (or the equivalent failing step while iterating) |
| Dogfood or module splits | `./scripts/verify.sh full` (or `/verify`) |
| Merge-ready claim | `/verify` full (`deny`, `audit`, dry-rs `findings=0`) |

- Do not claim a check passed unless it was actually run and passed.
- If verification cannot be completed, clearly state what was not run and why.
