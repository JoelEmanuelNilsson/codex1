# Round 12 Heavy Review Findings

Date: 2026-04-21

Baseline under review: `9421cf8 round-11 repairs: harden plan/outcome contracts`

This round deployed 16 read-only reviewer lanes using `gpt-5.4` with high reasoning. Every reviewer was instructed to read `README.md` and every markdown file under `docs/**/*.md`, pay special attention to `docs/codex1-rebuild-handoff/` as the primary intended-state source, use prior audit decisions as supporting context, mutate no repo mission state, and report only verified P0/P1/P2 findings.

Baseline verification before review:

- `cargo fmt --check` passed.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo test` passed.
- `make verify-contract` passed.

## Summary

- P0: 0
- P1: 12
- P2: 18

Raw reviewer counts before dedupe: 30 findings.

## Reviewer Management Table

| Reviewer ID | Assigned surface | Model | Reasoning | Execution wave | Result | P0 | P1 | P2 | Finding titles | Evidence quality | Duplicate/unique guess | Repro status | Main-thread initial disposition | Recommended next step |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- |
| R01 | CLI contract and envelopes | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 3 | `task next` close drift; `close record-review` wrong missing-file contract; terminal mission-close late results dropped | High | First overlaps R16; latter two likely round-11 continuations | Reproduced | Candidate P2s | Shard with close/contract reviewers |
| R02 | Mission resolution and path security | `gpt-5.4` | high | 1 | `NONE` | 0 | 0 | 0 | None | High | No verified issues | N/A | No finding | No action |
| R03 | Outcome and clarify | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 1 | Malformed OUTCOME domains still ratify | High | Overlaps R16 outcome validator regression | Reproduced | Candidate P2 | Shard with validator reviewers |
| R04 | Plan validation | `gpt-5.4` | high | 1 | Findings | 0 | 2 | 1 | Locked-plan replacement without replan; historical review-ID reuse; `PLAN.yaml` mission mismatch | High | All look distinct; mission mismatch overlaps R16 | Reproduced | Candidate P1/P2s | Shard with plan-truth reviewers |
| R05 | DAG, waves, and graph | `gpt-5.4` | high | 1 | Findings | 0 | 3 | 0 | Dirty review advertised before repair; post-repair status still blocked; superseded dirty review still blocks | High | Strong overlap with round-11 F09/F14 families | Reproduced | Candidate P1s | Shard with readiness/review reviewers |
| R06 | Task lifecycle | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 2 | `task start` bypasses replan gate; `task next` still points at review before repair; repaired target can reopen | High | All overlap known readiness/replan families | Reproduced | Candidate merge/accept set | Shard with task/replan reviewers |
| R07 | Review lifecycle | `gpt-5.4` | high | 1 | Findings | 0 | 2 | 1 | Restarted review boundary still accepts stale old results; superseded dirty reviews still current; unlocked replan late results fail `PLAN_INVALID` | High | First likely round-11 F19; others overlap known families | Reproduced | Candidate P1/P2s | Shard with review-boundary reviewers |
| R08 | Replan lifecycle | `gpt-5.4` | high | 1 | Findings | 0 | 2 | 1 | Superseded dirty reviews block rebuilt DAG; restarted boundaries still count stale results; unlocked replan late results fail `PLAN_INVALID` | High | Substantial overlap with R07 | Reproduced | Candidate merge set | Shard with review-boundary reviewers |
| R09 | Close lifecycle | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 1 | Replan preserves passed mission-close review; readiness ignores `CLOSEOUT.md` writability | High | Both appear distinct | Reproduced | Candidate P1/P2 | Shard with close/readiness reviewers |
| R10 | State persistence and concurrency | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 1 | Artifact files can publish before failed event/state commit; `plan scaffold` logs success before `PLAN.yaml` is writable | High | Both appear distinct | Reproduced | Candidate P1/P2 | Shard with persistence reviewers |
| R11 | Status and Ralph | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 1 | Bare `status` degrades zero-candidate `PLANS/` to `foundation_only` fallback | High | Unique follow-on to round-11 status work | Reproduced | Candidate P2 | Shard with status/install reviewers |
| R12 | Loop commands and orchestration skills | `gpt-5.4` | high | 1 | `NONE` | 0 | 0 | 0 | None | High | No verified issues | N/A | No finding | No action |
| R13 | Install and Makefile UX | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 1 | Relative `INSTALL_DIR` breaks `verify-installed` | High | Unique | Reproduced | Candidate P2 | Shard with install/docs reviewers |
| R14 | Test adequacy | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 2 | No test for dirty-then-clean mission-close history bug; no test for missing close findings-file contract bug | High | Both reinforce live runtime bugs rather than stand alone | Reproduced | Merge as test requirements | Fold into runtime findings |
| R15 | Docs and handoff cross-check | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 2 | `review status` docs wrong; `close record-review` docs drift on success and error | High | First likely round-11 docs continuation; second overlaps missing-file family plus new success drift | Reproduced | Candidate P2s / likely merge | Shard with docs reviewers |
| R16 | Current diff and regression review | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 3 | `task next` close drift; `PLAN.yaml` mission mismatch; OUTCOME junk-list ratification | High | First overlaps R01; others overlap R03/R04 | Reproduced | Candidate merge set | Shard with validator/close reviewers |

## Deduplicated Raw Finding Index

These are raw candidates for finding review. Severity here reflects the reporting reviewer severity, not the final accepted severity.

### F01 P2: `task next` advertises `close` even when proof-aware close blockers make `close complete` fail

Reported by R01 and R16.

Evidence:

- `task next` routes to `kind: "close"` from all-tasks-terminal plus `close.review_state == passed` without consulting the proof-aware close readiness used by `status` and `close check`.
- Live repro after deleting a required proof file produced:
  - `status`: `next_action.kind: "blocked"` and `close_ready: false`
  - `task next`: `kind: "close"`
  - `close check`: blocker `PROOF_MISSING`
  - `close complete`: `CLOSE_NOT_READY`

Initial disposition: candidate P2. Strong cross-surface contract contradiction.

### F02 P2: `close record-review --findings-file` still returns the wrong public error contract for a missing findings file

Reported by R01, reinforced by R14 and R15.

Evidence:

- `close::record_review` returns `PROOF_MISSING` with a `task finish` hint when the provided findings file is absent.
- Public docs do not list `PROOF_MISSING` for this surface.
- R14 confirmed there is no close-surface regression test covering this live bug.

Initial disposition: candidate P2. Likely a continuing round-11 family.

### F03 P2: Terminal mission-close review results are rejected instead of being audited as `contaminated_after_terminal`

Reported by R01.

Evidence:

- In a terminal mission, `close record-review --findings-file ...` returns `CLOSE_NOT_READY`.
- No stale/contaminated audit record is appended even though the contract docs describe `contaminated_after_terminal` as an audited late-output category.

Initial disposition: candidate P2.

### F04 P2: `outcome check` / `outcome ratify` still bless malformed OUTCOME field domains and non-string junk entries

Reported by R03 and R16.

Evidence:

- `status: stale` still passes validation because the checker only enforces non-empty string, not allowed domain.
- Required list fields such as `must_be_true`, `success_criteria`, `constraints`, `quality_bar`, and peers still pass when they contain only values like `1`, `false`, `{}`, or `[]`.
- `definitions: { foo: 5 }` also ratifies even though the intended shape is term-to-string meaning.

Initial disposition: candidate P2. Distinct from the round-11 empty-list repair.

### F05 P1: `plan check` lets a changed locked plan replace the live DAG without a replan, orphaning active work from readiness and close gating

Reported by R04.

Evidence:

- A locked plan with in-progress `T1` can be edited in place to a different locked DAG containing `T2`.
- A second `plan check` succeeds and overwrites `state.plan.task_ids` to `["T2"]` even though `T1` remains in progress.
- `status` and `close check` then reason only over the replacement DAG, allowing the mission to advance while the unsuperseded original work is still active.

Initial disposition: candidate P1. This looks like a core plan-lock violation.

### F06 P1: `plan check` still allows reuse of historical review task IDs when old truth only survives in `state.reviews`

Reported by R04.

Evidence:

- The round-11 guard rejects reuse found in `state.tasks`, but not review IDs whose old truth still exists in `state.reviews`.
- Reusing `T2` as fresh code work after a dirty review on old `T2` succeeded on relock.
- After relock, `status` simultaneously reported `verdict: "blocked"` and `next_action.kind: "run_task"` for the reused ID.

Initial disposition: candidate P1. Strong identity aliasing bug.

### F07 P2: `plan check` still does not verify `PLAN.yaml mission_id` against the active mission

Reported by R04 and R16.

Evidence:

- `PLAN.yaml` containing `mission_id: other-mission` still locked successfully for mission `demo`.
- Validation currently checks only that `mission_id` is present and non-empty.

Initial disposition: candidate P2.

### F08 P1: Dirty planned reviews are still advertised as runnable review work before repair is complete

Reported by R05 and reinforced by R06.

Evidence:

- With an accepted-current dirty review, `status` says repair is required, but `task next` returns `run_review`, `plan waves` advances to the review wave, and `plan graph` marks the review node ready.
- `review start` then fails with `REVIEW_FINDINGS_BLOCK`.
- R06 separately confirmed `task next` still points at the review instead of the repair step in the same family.

Initial disposition: candidate P1. Continuing round-11 F09 family unless finding review decides the latest evidence should stand separately.

### F09 P1: After repair is complete, `status` stays `blocked` even though the re-review is actually ready and startable

Reported by R05.

Evidence:

- Once the repaired task’s `finished_at` is newer than the dirty review, `review start` succeeds and `task next` returns `run_review`.
- `status.verdict` still remains `blocked`, which can stall orchestrators that gate on the top-level verdict.

Initial disposition: candidate P1. Likely merge into the stale dirty-review readiness family.

### F10 P1: Superseded dirty planned reviews still block the rebuilt DAG after replan and relock

Reported by R05, R07, and R08.

Evidence:

- After replan replaces the affected tasks and the replacement DAG is completed, `status` and `close check` still block on the old dirty review.
- `task next` can simultaneously advance to `mission_close_review`, creating contradictory public truth.

Initial disposition: candidate P1. Continuing round-11 F14 family.

### F11 P1: `task start` bypasses the replan gate and can execute stale-plan work after `replan.triggered=true`

Reported by R06.

Evidence:

- `status` and `task next` both return `replan` when `replan.triggered` is set.
- Despite that, `task start T1` still succeeds because it only checks `plan.locked`, not the active replan gate.

Initial disposition: candidate P1. Continuing round-11 F10 family.

### F12 P2: Late review results during the unlocked replan window still fail as `PLAN_INVALID` instead of being stale-audited

Reported by R07 and R08, reinforced by R14’s missing-test note.

Evidence:

- After `replan record` unlocks the plan, a late `review record` for the superseded boundary fails at the lock guard with `PLAN_INVALID`.
- The stale-review classification/audit path never runs, so late results are dropped instead of recorded as stale/superseded.

Initial disposition: candidate P2. Likely merge into the already-open stale-after-replan family.

### F13 P1: Restarting a planned review still does not fence off late results from the previous review boundary

Reported by R07 and R08.

Evidence:

- After a review is restarted for the same task ID, replaying the older result is still treated as current for the new boundary.
- Reviewers also observed that stale old results can still count toward the six-dirty replan trigger.

Initial disposition: candidate P1. Continuing round-11 F19 family.

### F14 P1: Side-effectful review/close mutations can publish artifacts before the event/state commit succeeds

Reported by R10.

Evidence:

- If `EVENTS.jsonl` is replaced with a directory, `review record`, `close record-review`, and `close complete` can all write outward-facing artifact files before `append_event` fails.
- The command then returns an error while `STATE.json` remains unchanged, leaving orphaned findings/closeout artifacts that were never committed as mission truth.

Initial disposition: candidate P1. Distinct persistence/corruption family.

### F15 P2: `plan scaffold` records success and bumps revision before confirming `PLAN.yaml` is writable

Reported by R10.

Evidence:

- Replacing `PLAN.yaml` with a directory makes `plan scaffold` fail after it already advanced `STATE.json.revision` and appended `plan.scaffold` to `EVENTS.jsonl`.
- No actual scaffold file is produced.

Initial disposition: candidate P2.

### F16 P2: Bare `status` incorrectly degrades an in-repo zero-candidate `PLANS/` tree into the Ralph `foundation_only` fallback

Reported by R11.

Evidence:

- In a repo whose cwd contains `PLANS/` but no missions, bare `status --json` exits `0` and returns `foundation_only`.
- The contract says the bare single-mission resolver should error on `0` or `>1` candidates once a `PLANS/` tree is present.

Initial disposition: candidate P2.

### F17 P2: `verify-installed` breaks the documented `INSTALL_DIR=<path>` flow when the install dir is relative

Reported by R13.

Evidence:

- `make install-local verify-installed INSTALL_DIR=.tmp-install-rel3` succeeds at install time but the verification target later `cd`s into `/tmp`.
- The relative `INSTALL_DIR` is then resolved against `/tmp`, so `command -v codex1` cannot find the installed binary even though the install succeeded.

Initial disposition: candidate P2.

### F18 P2: `CLOSEOUT.md` still lies about dirty-then-clean mission-close history

Reported by R14.

Evidence:

- After a dirty mission-close review followed by a clean one, `close complete` still writes `Clean on the first round.` to `CLOSEOUT.md`.
- R14 also verified there is no regression test covering this live bug.

Initial disposition: candidate P2. Continuing round-11 F18 family.

### F19 P2: `review status` docs still describe the wrong JSON shape

Reported by R15.

Evidence:

- Docs still show `record.task_id` and `targets` as a string array.
- The implementation emits no `record.task_id`, and `targets` is an array of objects containing `task_id`, `status`, and `consecutive_dirty`.

Initial disposition: candidate P2. Likely docs-only continuation of round-11 F02.

### F20 P2: `close record-review` docs no longer match either success or missing-file behavior

Reported by R15.

Evidence:

- Docs still show a stale success envelope and omit the currently emitted `PROOF_MISSING` error family.
- Live clean repro shows fields such as `reviewers`, `findings_file`, `consecutive_dirty`, `replan_triggered`, and `dry_run` that are absent from the docs.

Initial disposition: candidate P2. Likely merges partly with F02 plus remaining success-envelope docs drift.

### F21 P1: A passed mission-close review survives replan and can bless new work without a fresh terminal review

Reported by R09.

Evidence:

- `replan record` and the subsequent `plan check` relock leave `state.close.review_state == passed` untouched.
- After new work is added and completed on the rebuilt DAG, `status` and `close check` still report the mission as ready to close without requiring a new mission-close review.

Initial disposition: candidate P1. Distinct terminal-boundary integrity bug.

### F22 P2: `status` and `close check` can announce terminal readiness even when `close complete` is guaranteed to fail on `CLOSEOUT.md`

Reported by R09.

Evidence:

- Making `CLOSEOUT.md` a directory does not block `status` or `close check`.
- Both still report close readiness while `close complete` fails immediately on the unwritable closeout target.

Initial disposition: candidate P2. Distinct close-readiness contract gap.
