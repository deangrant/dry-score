# Verify

Read and follow the **verify-gates** skill at
[`.agents/skills/verify-gates/SKILL.md`](../skills/verify-gates/SKILL.md).

Run the workspace gate script from the repo root:

```bash
./scripts/verify.sh lite    # fmt + clippy + test
./scripts/verify.sh full    # default; includes deny, audit, dry-rs, llvm-cov, crap
./scripts/verify.sh         # same as full
```

Print failures clearly. Do **not** relax clippy/CRAP thresholds or edit
`dry.toml` solely to hide findings. Fix root causes, then re-run until green.
