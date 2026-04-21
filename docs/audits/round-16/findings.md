# Round 16 Heavy Review Findings

Date: 2026-04-22

Baseline under review: `9619b632e07c28be6f32e52e10ca4ff46d8b68c0`

This round deployed 16 read-only reviewer lanes using `gpt-5.4` with high reasoning. Every reviewer was instructed to read `README.md` and every markdown file under `docs/**/*.md`, pay special attention to `docs/codex1-rebuild-handoff/` as the primary intended-state source, use prior audit decisions as supporting context, mutate no repo mission state, and report only verified P0/P1/P2 findings after trying to disprove each candidate.

## Summary

- P0: 0
- P1: 20
- P2: 13

Raw reviewer counts before dedupe: 33 findings.

## Reviewer Management Table

| Reviewer ID | Assigned surface | Model | Reasoning | Result | P0 | P1 | P2 | Finding titles | Evidence quality | Duplicate/unique guess | Repro status | Main-thread initial disposition | Recommended next step |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- |
| R01 | CLI contract and envelopes | `gpt-5.4` | high | Findings | 0 | 0 | 1 | Same-seq event recovery can commit state under a stale event | High | Duplicate with R10/R16 | Reproduced | Candidate P2 | State-transaction shard |
| R02 | Mission resolution and path security | `gpt-5.4` | high | Findings | 0 | 2 | 0 | Dirty review state commits after rejected review artifact path; close artifacts publish without committed state | High | Duplicates R10/R16/R09 families | Reproduced | Candidate P1s | Artifact transaction shard |
| R03 | Outcome and clarify | `gpt-5.4` | high | Findings | 0 | 1 | 1 | Failed ratify can leave OUTCOME.md ratified while STATE remains draft; check accepts YAML status spellings ratify cannot rewrite | High | First duplicate with R10/R16; second unique | Reproduced | Candidate P1/P2 | Outcome shard |
| R04 | Plan validation | `gpt-5.4` | high | Findings | 0 | 2 | 1 | Replan relock can orphan active tasks and close; superseded dirty review blocks rebuilt DAG; hard-plan evidence missing summary | High | First likely unique, second duplicate with R08, third unique | Reproduced | Candidate P1/P2s | Plan/replan shard |
| R05 | DAG, waves, and graph | `gpt-5.4` | high | Findings | 0 | 1 | 0 | Repaired dirty reviews still split status and task-next | High | Duplicate with R06/R11 | Reproduced | Candidate P1 | Readiness convergence shard |
| R06 | Task lifecycle | `gpt-5.4` | high | Findings | 0 | 2 | 0 | `task finish` mutates after mandatory replan; repaired dirty-review targets do not converge to re-review | High | First unique; second duplicate with R05/R11 | Reproduced | Candidate P1s | Task/readiness shard |
| R07 | Review lifecycle | `gpt-5.4` | high | Findings | 0 | 2 | 1 | Restarted review accepts stale findings; unlocked-replan stale audit lacks category; duplicate mission-close dirties trigger replan | High | First duplicate with R14, third duplicate with R08/R12 | Reproduced | Candidate P1/P2s | Review/close shard |
| R08 | Replan lifecycle | `gpt-5.4` | high | Findings | 0 | 3 | 1 | Mission-close dirty can be marked clean; superseded dirty review blocks rebuilt DAG; post-relock old reviews return PLAN_INVALID; dirty review artifact commits before artifact | High | Multiple duplicate families | Reproduced | Candidate P1/P2s | Replan/review shard |
| R09 | Close lifecycle | `gpt-5.4` | high | Findings | 0 | 2 | 0 | Close artifacts publish before state/event commit; mission-close review records while locked plan invalid | High | First duplicate with R02/R10; second likely unique | Reproduced | Candidate P1s | Close shard |
| R10 | State persistence and concurrency | `gpt-5.4` | high | Findings | 0 | 2 | 1 | Dirty review state commits before findings artifact; precommit artifact writes split OUTCOME/STATE; same-seq event recovery attaches wrong audit event | High | Duplicates R02/R03/R16 | Reproduced | Candidate P1/P2s | State-transaction shard |
| R11 | Status and Ralph | `gpt-5.4` | high | Findings | 0 | 1 | 1 | `task next` stuck on repair after repair; `invalid_state` still allows Stop | High | First duplicate with R05/R06; second unique | Reproduced | Candidate P1/P2 | Status/Ralph shard |
| R12 | Loop commands and orchestration skills | `gpt-5.4` | high | Findings | 0 | 1 | 1 | Mission-close dirty immediately clean without repair; terminal loop guard bypasses stale-writer conflict | High | First duplicate with R08; second unique | Reproduced | Candidate P1/P2 | Close/loop shard |
| R13 | Install and Makefile UX | `gpt-5.4` | high | Findings | 0 | 0 | 1 | Spaced `INSTALL_DIR` rewrites path and verify falsely passes | High | Historical install family | Reproduced | Candidate P2 | Install shard |
| R14 | Test adequacy | `gpt-5.4` | high | Findings | 0 | 3 | 1 | `task finish` terminal guard untested and missing; stale review replay untested and live; review artifact failure not covered and live; same-seq event mismatch untested | High | Duplicates runtime findings, test gaps | Reproduced | Candidate merge set | Test + implementation shards |
| R15 | Docs and handoff cross-check | `gpt-5.4` | high | Findings | 0 | 0 | 1 | Pure-YAML outcomes render blank closeout summaries | High | Unique pure-YAML follow-on | Reproduced | Candidate P2 | Closeout/outcome shard |
| R16 | Current diff and regression review | `gpt-5.4` | high | Findings | 0 | 2 | 1 | Failed review artifact write commits dirty state; outcome ratify precommit split; event idempotence reuses mismatched trailing event | High | Duplicates R02/R03/R10 | Reproduced | Candidate P1/P2s | Current-diff/state shard |

## Deduplicated Raw Finding Index

These are raw candidates for finding review. Severity here reflects the reporting reviewer severity, not the final accepted severity.

### F01 P2: Spaced `INSTALL_DIR` paths are rewritten, and `verify-installed` falsely passes

Reported by R13.

Evidence:

- `Makefile` still computes `INSTALL_DIR_ABS := $(abspath $(INSTALL_DIR))`.
- A quoted install dir containing spaces is split and rewritten into a different path.
- `install-local` and `verify-installed` both use the malformed value, so verification passes while the requested directory lacks `codex1`.

Initial disposition: candidate P2, historical custom-install family.

### F02 P1: `outcome ratify` can fail after rewriting `OUTCOME.md`, leaving state unratified

Reported by R03, R10, and R16.

Evidence:

- `outcome ratify` writes `OUTCOME.md` in a precommit hook before `append_event` and `STATE.json` persistence.
- With malformed or stale `EVENTS.jsonl`, ratify fails but `OUTCOME.md` has `status: ratified` while `STATE.json` remains draft.

Initial disposition: candidate P1, round-15 regression from state/file atomicity repair.

### F03 P2: `outcome check` accepts valid YAML status-key spellings that ratify cannot rewrite

Reported by R03.

Evidence:

- Validation parses YAML and accepts forms such as `"status": draft` or `status : draft`.
- Ratify uses a line scanner that recognizes only a literal `status:` prefix and then returns `OUTCOME_INCOMPLETE`.

Initial disposition: candidate P2 check/ratify disagreement.

### F04 P1: Mission-close dirty findings can be immediately marked clean without repair

Reported by R08 and R12.

Evidence:

- After `close record-review --findings-file`, `close.review_state` becomes `open`.
- `close record-review --clean` accepts `mission_close_review_open` and sets `passed` with no repair, replan, or new boundary.
- `close check` then reports ready.

Initial disposition: candidate P1, mission-close gate bypass.

### F05 P2: Terminal loop guard bypasses stale-writer conflict reporting

Reported by R12.

Evidence:

- `loop` transitions call `require_not_terminal` before `check_expected_revision`.
- A stale writer against a terminal mission gets `TERMINAL_ALREADY_COMPLETE` instead of retryable `REVISION_CONFLICT`.

Initial disposition: candidate P2 stale-writer contract inconsistency.

### F06 P1: Repaired dirty reviews still split status, readiness, and `task next`

Reported by R05, R06, R11, and R14.

Evidence:

- After dirty planned-review findings are repaired with a newer `finished_at`, `status.next_action.kind` can advance to `run_review`.
- `status.verdict` remains `blocked`.
- `task next` still returns `repair` because it checks only `AwaitingReview`, not whether the target was repaired after the dirty review timestamp.

Initial disposition: candidate P1, core orchestration mismatch.

### F07 P1: `task finish` can mutate stale-plan work after a mandatory replan is triggered

Reported by R06.

Evidence:

- `task finish` preflights and rechecks only `require_plan_locked`, not `require_executable_plan`.
- With `state.replan.triggered=true`, `task next` reports `replan`, but an in-progress task can still be finished.
- The task can become `complete`, after which `replan record --supersedes` refuses to supersede it.

Initial disposition: candidate P1.

### F08 P1: Close artifacts can publish before state/event commit succeeds

Reported by R02 and R09.

Evidence:

- `close complete` writes `CLOSEOUT.md` in a precommit callback before event/state persistence.
- `close record-review --findings-file` writes `reviews/mission-close-<rev>.md` inside the state mutator before event/state commit.
- Failures after publication leave close artifacts without matching committed truth.

Initial disposition: candidate P1, close artifact transaction family.

### F09 P1: Mission-close review can be recorded while the locked plan is invalid

Reported by R09.

Evidence:

- `close record-review` gates on `ReadinessReport::from_state_and_paths`, which does not enforce locked-plan hash drift.
- `status` reports `invalid_state` and `close check` returns `PLAN_INVALID`, but `close record-review --clean` can still set `close.review_state = passed`.
- Restoring the plan later makes `close_ready` true without rerunning mission-close review under a valid plan.

Initial disposition: candidate P1.

### F10 P1: Restarted planned-review boundaries still accept stale findings from the prior round

Reported by R07 and R14.

Evidence:

- `review start` overwrites the prior review record with fresh `Pending` and a new `boundary_revision`.
- `classify()` treats pending records as `accepted_current`.
- Replaying an old external findings file after repair and restart returns `accepted_current` and increments dirty counters.

Initial disposition: candidate P1, continuation of planned-review boundary family.

### F11 P2: Late planned-review output during unlocked replan is audited without stable stale category/targets

Reported by R07.

Evidence:

- Round-15 changed unlocked-plan late review output from `PLAN_INVALID` to `review.stale`.
- The event payload has `targets: []` and omits `category: "stale_superseded"`.
- Consumers cannot distinguish stable late-output classes from the audit log.

Initial disposition: candidate P2.

### F12 P1: Duplicate mission-close dirty records count as six rounds and trigger replan

Reported by R07 and R14.

Evidence:

- Mission-close review state has no durable round identity.
- Repeating the same dirty findings file six times while `review_state=open` increments `__mission_close__` to 6 and triggers replan.

Initial disposition: candidate P1, mission-close round-identity family.

### F13 P2: `phase` is still documented as a live review/mission-close state machine

Reported by R15.

Evidence:

- Docs describe transitions through `review_loop` and `mission_close`.
- Runtime leaves `phase` as `execute` until terminal close; actionable progression is expressed through `verdict` and `next_action`.

Initial disposition: candidate P2 docs/runtime drift.

### F14 P2: CLI docs still publish PascalCase review/task status values

Reported by R15.

Evidence:

- `docs/cli-reference.md` still includes JSON examples with values such as `Dirty` and `AwaitingReview`.
- Runtime emits `dirty`, `accepted_current`, `awaiting_review`, and other snake_case/lowercase strings.

Initial disposition: candidate P2 docs drift.

### F15 P2: Docs promise externally recorded absolute proof paths remain absolute, but review packets relativize repo-local external proofs

Reported by R15.

Evidence:

- Docs state externally recorded absolute proof paths remain absolute.
- `review packet` resolves proof paths and always uses `relative_from_repo` for paths under the repo root.
- A mission-external absolute proof under the repository is emitted as a relative path.

Initial disposition: candidate P2.

### F16 P2: Same-sequence event recovery can attach the wrong audit event to a new mutation

Reported by R01, R10, R14, and R16.

Evidence:

- `append_event` treats any existing trailing event with the same `seq` as an idempotent retry.
- It does not check whether the existing event kind/payload matches the mutation being committed.
- A different command can advance `STATE.json` while the stale prior event remains the only audit entry for that sequence.

Initial disposition: candidate P2, round-15 regression.

### F17 P1: Replan relock can orphan active tasks and still allow terminal close

Reported by R04.

Evidence:

- `replan record` can be run without `--supersedes`.
- `plan check` accepts a replacement-only DAG and overwrites `state.plan.task_ids`.
- `close check` iterates only `state.plan.task_ids`, so old non-superseded `in_progress` tasks outside the relocked snapshot can be ignored.

Initial disposition: candidate P1.

### F18 P1: Superseded dirty planned-review truth survives replan/relock and blocks the rebuilt DAG

Reported by R04 and R08.

Evidence:

- `replan record` supersedes tasks but does not retire or reclassify `state.reviews`.
- `readiness::has_current_dirty_review` and `close check` block on any accepted-current dirty review, regardless of current DAG membership.
- A replacement plan can execute but remain blocked by obsolete dirty review truth.

Initial disposition: candidate P1, stale dirty review truth family.

### F19 P2: Hard-plan evidence can be locked without an evidence summary

Reported by R04.

Evidence:

- Hard-plan gate counts `planning_process.evidence[].kind`.
- It does not validate that evidence entries include a non-empty `summary`.
- A hard plan with `evidence: [{kind: explorer}]` can lock.

Initial disposition: candidate P2.

### F20 P1: Dirty planned-review state commits before the findings artifact is durable

Reported by R02, R08, R10, R14, and R16.

Evidence:

- Round-15 moved planned-review findings artifact writing after `state::mutate_dynamic`.
- If artifact write fails, command returns error but `STATE.json` and `EVENTS.jsonl` already record accepted-current dirty review truth and `findings_file`.
- Retry can classify as late and fail to restore the missing artifact.

Initial disposition: candidate P1, round-15 regression from artifact-ordering repair.

### F21 P2: Pure-YAML outcomes render blank closeout summaries

Reported by R15.

Evidence:

- Pure-YAML `OUTCOME.md` is now accepted and ratified.
- `closeout.rs` extracts `interpreted_destination` only from fenced frontmatter.
- Terminal `CLOSEOUT.md` renders `_interpreted_destination not found in OUTCOME.md_` for a valid pure-YAML mission.

Initial disposition: candidate P2.

### F22 P1: `task finish` can mutate terminal missions

Reported by R14.

Evidence:

- Round-15 added a terminal guard through `require_executable_plan`, but `task finish` uses only the weaker locked-plan guard.
- A terminal mission with an in-progress task can still run `task finish`, bump revision, and mark the task complete.

Initial disposition: candidate P1.

### F23 P2: Old review outputs after replan relock return `PLAN_INVALID` instead of stale audit

Reported by R08.

Evidence:

- The unlocked-replan stale special-case does not cover output that arrives after relock.
- `review record` looks up the review task in the current replacement plan before classification.
- A late result for an old superseded review task missing from the new DAG returns `PLAN_INVALID` and appends no `review.stale` event.

Initial disposition: candidate P2.

### F24 P2: `invalid_state` status can still set `stop.allow: true`

Reported by R11.

Evidence:

- `status` builds an `invalid_state` projection for locked-plan hash drift.
- It calls `readiness::stop_allowed(state)` using the raw state rather than the emitted invalid verdict.
- With active unpaused loop plus locked-plan drift, `status` can emit `verdict: "invalid_state"` and `stop.allow: true`; Ralph then exits 0.

Initial disposition: candidate P2.
