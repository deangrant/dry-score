# Dry dogfood

Read and follow the **dry-dogfood** skill at
[`.agents/skills/dry-dogfood/SKILL.md`](../skills/dry-dogfood/SKILL.md).

Scan this repo with dry-rs, classify each cluster (dedupe vs intentional
`// dry-rs:ignore` vs merge tests), fix until `findings=0`, then confirm crap
still passes via `./scripts/verify.sh full`. Do not re-merge CC≤5 helpers or
relax thresholds / `dry.toml` to hide findings.
