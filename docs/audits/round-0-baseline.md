# Round 0 — Baseline Hygiene

**Loop:** Codex1 v3 review → fix convergence loop.
**Purpose of this file:** record the baseline commit that Round 1 reviewers will audit, and prove it is green end-to-end before any reviewer touches it.

## Fast-forward

- Before: `2ef3ce7` (`iter3-fix: F9/F10/F11 (clippy + missed fixtures + upgrade trap)`)
- After:  `c5e07ad` (`iter4-fix-followup: regression test for F11 upgrade trap`)
- Strategy: `git pull --ff-only origin main` (no merge, no rebase).
- Commits pulled in order:
  1. `5a16894` iter3-wave-fix: close/check blocker enum now walks plan.task_ids
  2. `e5f6bf4` iter4 audit: cli-contract + skills + handoff (post-5a16894)
  3. `b212ca8` iter4-fix: actually tighten plan-check upgrade guard (P1 F11)
  4. `c5e07ad` iter4-fix-followup: regression test for F11 upgrade trap

## Baseline validation (all must pass to proceed)

| Check | Result |
|-------|--------|
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS (no warnings) |
| `cargo test` (full suite, 19 test binaries) | PASS — 170 tests passed, 0 failed |

Per-binary test counts (in order they ran):
```
10  0  17  1  5  1  10  20  8  13  10  8  6  10  16  14  3  18  0
```

## Round 1 starts from here

Reviewer worktrees will branch from `c5e07ad`. Fix-agent commits land on top of `c5e07ad` on `main`.
