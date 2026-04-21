# Round 17 Heavy Review Findings

Date: 2026-04-22

Baseline under review: `d4111e1f0b79513b6ad4a43eb14b440843c1ea40`

This round deployed 16 read-only reviewer lanes using `gpt-5.4` with high reasoning. Every reviewer was instructed to read `README.md` and every markdown file under `docs/**/*.md`, pay special attention to `docs/codex1-rebuild-handoff/` as the primary intended-state source, use prior audit decisions as supporting context, mutate no repo mission state, and report only verified P0/P1/P2 findings after trying to disprove each candidate.

## Summary

- P0: 0
- P1: 18
- P2: 6

Raw reviewer counts before dedupe: 24 findings.

## Reviewer Management Table

| Reviewer ID | Assigned surface | Model | Reasoning | Result | P0 | P1 | P2 | Finding titles | Evidence quality | Duplicate/unique guess | Repro status | Main-thread initial disposition | Recommended next step |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- |
| R01 | CLI contract and envelopes | `gpt-5.4` | high | Findings | 0 | 1 | 0 | `mission_close_review_open` can no longer be closed | High | Duplicate with R09/R12/R16 | Reproduced | Candidate P1 | Close lifecycle shard |
| R02 | Mission resolution and path security | `gpt-5.4` | high | Findings | 0 | 0 | 1 | Symlinked `reviews/` poisons closeout history | High | Historical closeout-history family | Reproduced | Candidate P2 | Path/closeout shard |
| R03 | Outcome and clarify | `gpt-5.4` | high | Findings | 0 | 0 | 1 | Ratify can rewrite OUTCOME before event/state commit failure | High | Outcome atomicity family | Reproduced | Candidate P2 | Outcome transaction shard |
| R04 | Plan validation | `gpt-5.4` | high | Findings | 0 | 1 | 0 | Replan relock can strand omitted active tasks outside locked DAG | High | Duplicate with R06 | Reproduced | Candidate P1 | Replan/readiness shard |
| R05 | DAG, waves, and graph | `gpt-5.4` | high | Findings | 0 | 1 | 1 | Repaired dirty reviews leave status/waves/graph globally blocked; non-target awaiting-review deps satisfy review readiness | High | First duplicate with R06/R11/R16; second unique | Reproduced | Candidate P1/P2 | Readiness/DAG shard |
| R06 | Task lifecycle | `gpt-5.4` | high | Findings | 0 | 2 | 1 | Orphaned pre-replan tasks continue; dirty repair blocking scoped globally; non-target review deps over-admitted | High | Merge with R04/R05 | Reproduced | Candidate P1/P2s | Task/readiness shard |
| R07 | Review lifecycle | `gpt-5.4` | high | Findings | 0 | 3 | 0 | Planned review artifact can commit without artifact; mission-close dirty state stuck open; restarted review accepts stale outputs | High | Duplicates R10/R15/R16 families | Reproduced | Candidate P1s | Review/close shard |
| R08 | Replan lifecycle | `gpt-5.4` | high | Findings | 0 | 3 | 0 | Mission-close dirty cannot review clean; restarted reviews accept stale prior-boundary findings; replan rejects append-style plans keeping completed prerequisites | High | Third likely unique | Reproduced | Candidate P1s | Replan/review shard |
| R09 | Close lifecycle | `gpt-5.4` | high | Findings | 0 | 1 | 0 | Mission-close dirty review deadlocks documented loop | High | Duplicate with R01/R12/R16 | Reproduced | Candidate P1 | Close shard |
| R10 | State persistence and concurrency | `gpt-5.4` | high | Findings | 0 | 2 | 0 | Planned dirty review commits before artifact; mission-close dirty artifact stale revision under concurrency | High | First duplicate, second unique | Reproduced | Candidate P1s | State/artifact shard |
| R11 | Status and Ralph | `gpt-5.4` | high | Findings | 0 | 1 | 0 | Repaired dirty review still blocks status verdict | High | Duplicate with R05/R06/R16 | Reproduced | Candidate P1 | Status/readiness shard |
| R12 | Loop commands and orchestration skills | `gpt-5.4` | high | Findings | 0 | 1 | 0 | Mission-close open state cannot ever pass | High | Duplicate with R01/R09/R16 | Reproduced | Candidate P1 | Close/skills shard |
| R13 | Install and Makefile UX | `gpt-5.4` | high | `NONE` | 0 | 0 | 0 | None | High | No verified issues | N/A | No action | None |
| R14 | Test adequacy | `gpt-5.4` | high | Findings | 0 | 1 | 3 | Artifact transaction tests overfit; orphan-task close gate untested; stale-review audit test weak; active invalid-state Stop untested | High | Mostly test gaps backing live findings | Reproduced/inspected | Candidate merge set | Test adequacy shard |
| R15 | Docs and handoff cross-check | `gpt-5.4` | high | Findings | 0 | 1 | 0 | Dirty review commands can commit state before findings artifact | High | Duplicate with R07/R10 | Reproduced | Candidate P1 | Artifact shard |
| R16 | Current diff and regression review | `gpt-5.4` | high | Findings | 0 | 3 | 0 | Repaired dirty review verdict split; restarted reviews accept stale clean; mission-close dirty state points to failing action | High | Duplicates R05/R07/R09 families | Reproduced | Candidate P1s | Current-diff shard |

## Deduplicated Raw Finding Index

These are raw candidates for finding review. Severity here reflects the reporting reviewer severity, not the final accepted severity.

### F01 P1: Mission-close dirty review deadlocks the documented open-to-clean loop

Reported by R01, R07, R08, R09, R12, and R16.

Evidence:

- `close record-review --findings-file` sets `close.review_state = open`.
- `status` and skills route `mission_close_review_open` back to mission-close review.
- Both clean and dirty follow-up records reject `mission_close_review_open` inside the mutation path, returning `CLOSE_NOT_READY`.

Initial disposition: candidate P1.

### F02 P2: `close complete` still trusts a symlinked `reviews/` directory when rendering mission-close history

Reported by R02.

Evidence:

- `closeout.rs` counts `mission-close-*.md` via raw `read_dir(paths.reviews_dir())`.
- Replacing mission `reviews/` with a symlink to external `mission-close-*.md` files lets `CLOSEOUT.md` claim dirty rounds not in mission state.

Initial disposition: candidate P2, historical closeout-history path family.

### F03 P2: `outcome ratify` can still rewrite `OUTCOME.md` before event/state commit failure

Reported by R03.

Evidence:

- `outcome ratify` writes `OUTCOME.md` in `mutate_dynamic_with_precommit`.
- If later event append or state write fails, `OUTCOME.md` can say ratified while `STATE.json` remains unratified.

Initial disposition: candidate P2, outcome atomicity family.

### F04 P1: Replan relock can strand omitted active tasks outside the locked DAG

Reported by R04 and R06.

Evidence:

- `replan record` can run without superseding an in-progress old task.
- Replacement `plan check` can relock with only fresh IDs, leaving the old task outside `state.plan.task_ids`.
- `status` can be `blocked` while still advertising replacement work, and subsequent `replan record --supersedes <old>` rejects the old ID as unknown.

Initial disposition: candidate P1.

### F05 P1: Repaired dirty reviews still leave top-level status/readiness blocked while action surfaces advance to review

Reported by R05, R06, R11, and R16.

Evidence:

- After dirty review target repair, `status.next_action` and `task next` can return `run_review`.
- `derive_verdict` still returns `blocked` because the dirty review task itself remains live in the DAG.
- Waves/graph can also suppress ready review work under the global dirty blocker.

Initial disposition: candidate P1.

### F06 P2: Review readiness treats non-target `AwaitingReview` dependencies as satisfied

Reported by R05 and R06.

Evidence:

- Review task readiness allows every dependency to be `AwaitingReview`.
- A review targeting only `T2` but depending on `[T2, T3]` can start while non-target `T3` is awaiting its own review.

Initial disposition: candidate P2.

### F07 P1: Planned dirty review can commit state before its findings artifact is durable

Reported by R07, R10, R14, and R15.

Evidence:

- `review record --findings-file` commits state/event truth before writing `reviews/<id>.md`.
- If the artifact write fails after state commit, state points at a missing artifact and dirty counters advance.

Initial disposition: candidate P1.

### F08 P1: Restarted planned-review boundaries still accept stale prior-boundary outputs

Reported by R07, R08, and R16.

Evidence:

- `review start` writes a new pending record, but `review record` carries no boundary token.
- Old dirty or clean outputs from a prior boundary can be accepted as `accepted_current` after restart.

Initial disposition: candidate P1, planned-review boundary family.

### F09 P1: Replan rejects append-style plans that keep completed prerequisites

Reported by R08.

Evidence:

- During unlocked replan, `plan check` rejects every task ID present in prior `state.tasks`, `state.reviews`, or `plan.task_ids`.
- A replacement plan that keeps completed `T1` as a prerequisite and appends `T4/T5` is rejected as historical ID reuse.

Initial disposition: candidate P1.

### F10 P1: Mission-close dirty artifact can be written under the wrong revision after concurrent mutation

Reported by R10.

Evidence:

- `close record-review --findings-file` computes artifact path from pre-lock `current.revision + 1`.
- If another mutation commits first, close review commits at a later revision but writes the artifact under the stale filename; the success envelope points at the later filename that does not exist.

Initial disposition: candidate P1.

### F11 P1: Round-16 artifact transaction tests are overfit to precommit/event failures

Reported by R14.

Evidence:

- Tests cover event/preflight failures but not post-state artifact write failures for review and mission-close findings.
- This test gap backs live findings F07 and F10.

Initial disposition: merge into artifact findings plus test coverage.

### F12 P2: Orphan-task close gate has no direct regression coverage

Reported by R14.

Evidence:

- No test directly exercises a non-terminal task omitted from `state.plan.task_ids`.
- This backs F04.

Initial disposition: merge into F04 test coverage.

### F13 P2: Stale-review audit tests do not assert required category/target payload

Reported by R14.

Evidence:

- Existing stale-review test checks only that `review.stale` appears.
- It does not assert `payload.category` or original targets.

Initial disposition: candidate P2 test adequacy or merge into stale-review repair if accepted.

### F14 P2: Active invalid-state Stop behavior is not directly covered

Reported by R14.

Evidence:

- Existing status drift test does not set active/unpaused loop or assert `stop.allow=false`.

Initial disposition: likely test-only merge if touching status/Ralph.

### F15 P1: Dirty mission-close review commands can commit state before findings artifact durability

Reported by R15 and R10.

Evidence:

- `close record-review --findings-file` commits state/event truth before writing `reviews/mission-close-<rev>.md`.
- If artifact write fails after state commit, state records `review_state=open` and dirty counter without durable artifact.

Initial disposition: candidate P1, related to F10.

