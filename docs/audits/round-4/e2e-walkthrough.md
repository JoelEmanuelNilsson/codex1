# Round 4 — e2e-walkthrough audit

HEAD: `703a171` (round-3 fixes).

Reviewer: e2e-walkthrough (2/6). Lens: drive full missions end-to-end via `~/.local/bin/codex1` and verify every documented happy/failure path, plus the round-2/round-3 regression set. Prior-round decisions consumed: `round-1/decisions.md`, `round-2/decisions.md`, `round-3/decisions.md`. REJECTed findings not re-surfaced.

## Summary

Built the binary with `make -C /Users/joel/codex1 install-local` (release, clean compile, installed to `~/.local/bin/codex1`). Drove two missions from `/tmp/codex1-review-e2e-r4/` to cover happy path and failure modes, then cleaned up.

**Happy path verified.** `demo` mission went `init → hand-written OUTCOME.md (no blank line after closing fence) → outcome check → outcome ratify (rev 0→1) → outcome ratify again (rev 1→2) → outcome check re-parses OK → plan choose-level --level light (rev 3) → plan scaffold (rev 4) → PLAN.yaml authored with T1,T2,T3(review) → plan check (rev 5, `locked:true, plan_hash:sha256:9262dd24…`) → plan graph (mermaid output) → plan waves (W1:[T1,T2], W2:[T3]) → loop activate (rev 6) → task start T1 (rev 7) → task packet T1 (ok) → task finish T1 (rev 8) → task start T2 (rev 9) → task finish T2 (rev 10) → task start T3 (rev 11) → review packet T3 (ok except `mission_summary` leak — see P2-1) → review start T3 (rev 12) → review record T3 --clean (rev 13) → close check (dirty pending) → close record-review --findings-file (rev 14, `verdict:dirty, review_state:open`) → close record-review --clean (rev 15, `verdict:clean, review_state:passed`) → close check (ready:true) → close complete (rev 16, CLOSEOUT.md written, `verdict:terminal_complete, phase:terminal, stop.allow:true, reason:terminal`)`. Revision monotonicity held across all 16 bumps.

**Round-3 regression checks — all pass.**

1. **`outcome ratify` preserves OUTCOME.md without a blank line after the closing fence.** Hand-wrote `/tmp/codex1-review-e2e-r4/PLANS/demo/OUTCOME.md` where the closing `---` fence was directly followed by `# OUTCOME\n` (no blank line between). First ratify succeeded; standalone `---` fence line remained intact and the body heading stayed on its own line. Confirmed via `cat /tmp/codex1-review-e2e-r4/PLANS/demo/OUTCOME.md` — no `---# OUTCOME` collapsed form. Round-3 e2e P1-1 fix holds.
2. **Idempotent replay preserves parseability.** Second `outcome ratify` on the same file (rev 1 → 2) succeeded; subsequent `outcome check` returned `{ok:true, ratifiable:true, missing_fields:[], placeholders:[]}`. Round-3 e2e P1-1 reproducer B is now negative — the file survives repeated ratifies.
3. **`review-loop` skill dispatches on both verdicts.** `grep -n "ready_for_mission_close_review\|mission_close_review_open" /Users/joel/codex1/.codex/skills/review-loop/SKILL.md` returns lines 40 and 41 — both verdicts explicitly routed. Round-3 skills P1-1 fix holds.
4. **Dirty mission-close → clean record → terminal.** Walked the path live on the `demo` mission. Dirty record (`close record-review --findings-file /tmp/codex1-review-e2e-r4/mc-findings.md`) flipped `review_state:"open"` and verdict became `mission_close_review_open` (rev 14). Subsequent status showed `next_action.kind:"mission_close_review"` with `command:"$review-loop"` and `hint:"Mission-close review is open."` — i.e. the autopilot-style re-entry path. Clean re-record (`close record-review --clean`) transitioned `review_state:"passed"` and verdict to `mission_close_review_passed` (rev 15). `close check` then returned `ready:true`; `close complete` reached `verdict:terminal_complete, phase:terminal` (rev 16), CLOSEOUT.md written.

**Round-2 regression checks — all pass.**

- **`replan.triggered` cleared after `plan check` relock.** On the `block` mission: seeded state with `task start T2` (rev 11) → `replan record --reason scope_change --supersedes T2` (rev 12) → STATE showed `replan.triggered:true, triggered_reason:"scope_change", plan.locked:false`. Then `plan check` (rev 13) → STATE showed `replan.triggered:false` and `triggered_reason` absent (serde skip), `plan.locked:true`. Round-2 e2e P0-1 fix holds.
- **`task next` short-circuits correctly.** On `block` after the replan-record above (rev 12, `plan.locked:false, replan.triggered:true`), `task next` returned `{next: {kind:"plan", hint:"Draft and lock PLAN.yaml."}}`. The `!plan.locked` branch (cli/task/next.rs:26-39) wins over the `replan.triggered` branch at line 40 because `replan record` clears `plan.locked` — which is the canonical path; the `replan.triggered` branch is reached only when the dirty-6 auto-trigger fires while `plan.locked` is still true (the `task_next_replan_triggered_emits_replan_kind` unit test covers that). Round-2 e2e P2-1 fix holds.
- **TOCTOU concurrent replan/task-start.** `cargo test --release -p codex1 --test foundation concurrent_replan_and_task_start_preserves_plan_locked_invariant` → 1 passed, 0 failed, 0.28s. Round-2 correctness P1-1 fix holds.

**Less-tested CLI paths exercised.**

- `plan graph` → emits valid mermaid flowchart with class overlays (`complete/ready/blocked/…`), ok:true.
- `plan waves` → emits wave list with `wave_id, tasks, parallel_safe, blockers`; `current_ready_wave:"W1"`; `all_tasks_complete:false` before completion.
- `loop activate/pause/resume/deactivate` — all four transitions succeed, each bumping revision. Stop-hook parity confirmed below.
- `task packet T1` → emits `worker_instructions, write_paths, proof_commands, spec_excerpt, mission_summary` (all populated; `mission_summary` via `serde_yaml::from_str` — clean).
- `review packet T3` → emits `review_profile, profiles, targets, target_specs, proofs, diffs, mission_summary, reviewer_instructions`. **`mission_summary` is leaked as `"|\nShip a minimal mission that reaches terminal_complete."` — see P2-1.**

**Ralph stop hook (`scripts/ralph-stop-hook.sh`, reads `codex1 status --json` unaware of mission id).**

- Active loop, unpaused, mission mid-execution (`block` at rev 11 with T1 in_progress) → `stop.allow:false, reason:active_loop`. Hook exited 2. stderr: `ralph-stop-hook: blocking Stop - reason=active_loop`.
- Loop paused → `stop.allow:true, reason:paused`. Hook exited 0.
- Loop resumed → hook exited 2 again.
- Loop deactivated → hook exited 0 (`reason:idle`).
- Terminal mission (`demo` post-close) → hook exited 0 (`reason:terminal`).

**Error-envelope codes exercised end-to-end.** Observed (from this run): `OUTCOME_INCOMPLETE`, `OUTCOME_NOT_RATIFIED` (implicit), `PLAN_INVALID` (three flavors: missing spec file, bad task-id `R1`, review-loop-deadlock, unlocked plan on task/start, bad YAML), `DAG_MISSING_DEP` (via plan-check), `PROOF_MISSING`, `TASK_NOT_READY` (two forms: in_progress target on review start; Complete task on task finish), `REVISION_CONFLICT` (expected 999 vs actual 13), `CLOSE_NOT_READY` (via close check blockers list), `MISSION_NOT_FOUND` (implicit via bare `status` with two missions). Every envelope carried the expected `{ok:false, code, message, hint?, retryable, context?}` shape.

**One new P2 finding** — dedupe with round-4 cli-contract P2-1 (same bug, cross-reviewer overlap within the same round).

## P0

None.

## P1

None.

## P2

### P2-1 · `review packet` leaks YAML block-scalar indicator `|` into `mission_summary` (DEDUPE with round-4 cli-contract P2-1)

**Type:** (b) reproducible bug with exact command.

**Dedupe note.** Round-4 cli-contract audit P2-1 (`docs/audits/round-4/cli-contract.md:23`) identifies the same bug at the same severity. Filing under this lens for cross-reference; the e2e walkthrough independently reproduces it because `review packet` is on the happy path every skill-invoked mission walks through. Severity and disposition should be resolved once in the round-4 decisions ledger.

**Exact reproducer (live on this round's `demo` mission).**

```bash
cd /tmp/codex1-review-e2e-r4 && ~/.local/bin/codex1 review packet --mission demo T3 \
  | python3 -c "import json,sys;print(repr(json.load(sys.stdin)['data']['mission_summary']))"
# '|\nShip a minimal mission that reaches terminal_complete.'
```

For comparison, `task packet` on the same mission yields the clean string:

```bash
~/.local/bin/codex1 task packet --mission demo T1 \
  | python3 -c "import json,sys;print(repr(json.load(sys.stdin)['data']['mission_summary']))"
# 'Ship a minimal mission that reaches terminal_complete.'
```

**Code site.** `crates/codex1/src/cli/review/packet.rs:136-176` — `read_interpreted_destination` does a naive substring pull after `"interpreted_destination:"`, then iterates `rest.lines()`. The scaffold (`crates/codex1/src/cli/init.rs:82`) emits `interpreted_destination: |` on one line (the `|` is YAML's literal block-scalar indicator). After `raw.find(needle) + needle.len()`, `rest` begins with `" |\n  <body>..."`. The first line is ` |` (with a leading space from the YAML `: ` separator). The guard at line 163 checks `trimmed == "|"`, but `trimmed = line.trim_end()` only strips trailing whitespace — the leading space survives, so ` |` never matches `"|"`. The loop then pushes `trim_start()` of ` |` which is `|`, followed by the real body line, yielding the leaked `|\n<body>`.

Two sibling implementations of the same function parse correctly via `serde_yaml::from_str`:

- `crates/codex1/src/cli/task/worker_packet.rs:60-67` — used by `task packet`.
- `crates/codex1/src/cli/close/closeout.rs:136-142` — used by CLOSEOUT.md body.

Only `cli/review/packet.rs` diverges.

**Impact.** `review packet` is the reviewer subagent's prompt context (per `docs/codex1-rebuild-handoff/02-cli-contract.md:413-415`). The `mission_summary` is intended to be a short natural-language destination summary; reviewers receiving `|\n<body>` in a packet as the first line of context will parse the `|` literally or at best waste tokens parsing the glitch. Contract schema (`docs/cli-contract-schemas.md:264`) pins `mission_summary` as a string field; the schema does not prohibit `|`, but the task-packet analogue in the same doc (line 250) clearly intends the same destination-text semantic and emits it cleanly.

**Suggested fix.** Lift the `serde_yaml`-based `read_interpreted_destination` from `task/worker_packet.rs` (or `close/closeout.rs`) into a shared helper in `core/` or `cli/common/` and call it from all three sites. One-line regression test: assert `!mission_summary.starts_with('|')` and `!mission_summary.contains("|\n")` on a scaffolded-template mission packet.

## P3

None.

## Cleanup

- `/tmp/codex1-review-e2e-r4/` removed at end of run (see trailing cleanup in the invocation log).
