# Round 13 Heavy Review Findings

Date: 2026-04-21

Baseline under review: `f9e1a1a round-12 repairs: harden plan and close truth`

This round deployed 16 read-only reviewer lanes using `gpt-5.4` with high reasoning. Because the runtime capped live-agent concurrency, the 16 assignments were scheduled in rolling waves, but the assignment count remained the required 16 surfaces. Every reviewer was instructed to read `README.md` and every markdown file under `docs/**/*.md`, pay special attention to `docs/codex1-rebuild-handoff/` as the primary intended-state source, use prior audit decisions as supporting context, mutate no repo mission state, and report only verified P0/P1/P2 findings.

Baseline verification before review:

- `cargo fmt --check` passed.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo test` passed.
- `make verify-contract` passed.

## Summary

- P0: 0
- P1: 12
- P2: 16

Raw reviewer counts before dedupe: 28 findings.

## Reviewer Management Table

| Reviewer ID | Assigned surface | Model | Reasoning | Execution wave | Result | P0 | P1 | P2 | Finding titles | Evidence quality | Duplicate/unique guess | Repro status | Main-thread initial disposition | Recommended next step |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- |
| R01 | CLI contract and envelopes | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 3 | `close complete` idempotency drift; `review record` threshold docs drift; `status` empty-PLANS docs drift | High | First likely unique, latter docs items likely merge families | Reproduced | Candidate P2s | Close/docs shard |
| R02 | Mission resolution and path security | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 1 | Proof symlink escape; `close complete` precommit publish | High | First looks unique, second is known round-12 family | Reproduced | Candidate P2/P1 | Containment/persistence shard |
| R03 | Outcome and clarify | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 0 | Post-lock `OUTCOME.md` reratify changes live worker instructions | High | Unique | Reproduced | Candidate P1 | Outcome/lock shard |
| R04 | Plan validation | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 1 | Hard-plan downgrade bypass; unknown `review_profiles` lock | High | Both appear unique | Reproduced | Candidate P1/P2 | Plan-validation shard |
| R05 | DAG, waves, and graph | `gpt-5.4` | high | 1 | Findings | 0 | 2 | 1 | Dirty review reroute before repair; replan leaks ready work; superseded-dep wave collapse | High | First two merge old readiness families; third likely merge old superseded-wave family | Reproduced | Candidate merge set | Readiness/waves shard |
| R06 | Task lifecycle | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 2 | Restarted review stale current; stale-after-replan `PLAN_INVALID`; repaired target reopen | High | All three match already-open families | Reproduced | Candidate merge set | Review/task shard |
| R07 | Review lifecycle | `gpt-5.4` | high | 1 | Findings | 0 | 2 | 2 | Restart stale current; superseded dirty blocker; stale-after-replan; non-target awaiting-review dep passes | High | First three merge old families; dependency issue merges old F27 family | Reproduced | Candidate merge set | Review-boundary shard |
| R08 | Replan lifecycle | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 1 | Mission-close pass survives replan; terminal `replan record` mutates state | High | First merges old F21/F20 family; second looks unique | Reproduced | Candidate P2 + merge | Replan/terminal shard |
| R09 | Close lifecycle | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 0 | Mission-close pass survives replan | High | Duplicate of old terminal-boundary family | Reproduced | Merge target | Merge into terminal-boundary family |
| R10 | State persistence and concurrency | `gpt-5.4` | high | 2 | Findings | 0 | 2 | 1 | Restart stale current; mission-close pass survives replan; stale-after-replan `PLAN_INVALID` | High | All three match already-open families | Reproduced | Merge targets | Merge into lifecycle families |
| R11 | Status and Ralph | `gpt-5.4` | high | 2 | Findings | 0 | 2 | 1 | Dirty review advertised before repair; post-repair verdict still blocked; `close_ready` docs drift | High | First two merge old readiness family; docs item merges round-12 close-ready docs drift | Reproduced | Candidate merge set | Status/readiness shard |
| R12 | Loop commands and orchestration skills | `gpt-5.4` | high | 2 | `NONE` | 0 | 0 | 0 | None | High | No verified issues | N/A | No finding | No action |
| R13 | Install and Makefile UX | `gpt-5.4` | high | 2 | Findings | 0 | 0 | 2 | `verify-contract` flake; custom `INSTALL_DIR` docs recipe wrong | High | Both look unique | Reproduced | Candidate P2s | Install/docs shard |
| R14 | Test adequacy | `gpt-5.4` | high | 2 | `NONE` | 0 | 0 | 0 | None | High | No verified issues | N/A | No finding | No action |
| R15 | Docs and handoff cross-check | `gpt-5.4` | high | 2 | Findings | 0 | 0 | 1 | `replan record` docs scalar-vs-array drift | High | Likely unique docs-only drift | Reproduced | Candidate P2 | Docs shard |
| R16 | Current diff and regression review | `gpt-5.4` | high | 2 | Findings | 0 | 1 | 2 | `close complete` precommit publish; close-readiness permission gap; scaffold event-before-publish | High | First/third merge round-12 families; permission gap likely merges round-12 F22 family | Reproduced | Candidate merge set | Persistence/close shard |

## Deduplicated Raw Finding Index

These are raw candidates for finding review. Severity here reflects the reporting reviewer severity, not the final accepted severity.

### F01 P1: `outcome ratify` can re-ratify a changed `OUTCOME.md` after plan lock and silently change active worker instructions without a replan

Reported by R03.

Evidence:

- `outcome ratify` has no phase or plan-lock guard and still mutates `STATE.json` in execute phase.
- `task packet` re-reads `interpreted_destination` from `OUTCOME.md` on each packet build.
- Repro showed worker packet `mission_summary` changing after the second ratify while `status` still reported `phase: execute` and `plan_locked: true`.

Initial disposition: candidate P1. Looks unique and high-signal.

### F02 P1: `plan check` can downgrade a recorded hard-planning mission to `light` and bypass the hard-evidence lock gate

Reported by R04.

Evidence:

- `plan choose-level --level hard` recorded hard in state.
- `plan check` accepted `planning_level: { requested: hard, effective: light }` plus only `direct_reasoning` evidence.
- `STATE.json` ended locked with `requested_level: hard`, `effective_level: light`, and no hard evidence requirement enforced.

Initial disposition: candidate P1. Looks unique.

### F03 P2: `plan check` accepts unknown `review_profiles`, so invalid review tasks lock and later emit unusable profiles to `$review-loop`

Reported by R04.

Evidence:

- `plan check` validates review task targets/deps but not `review_profiles`.
- `review packet` forwards the invalid profile verbatim.
- Repro locked a review task with `review_profiles: [totally_invalid_profile]`, and the packet emitted that same unusable profile.

Initial disposition: candidate P2. Looks unique.

### F04 P2: `close complete` is not actually idempotent per the published contract

Reported by R01.

Evidence:

- Contract docs say subsequent calls return `TERMINAL_ALREADY_COMPLETE`.
- Implementation special-cases terminal missions with missing `CLOSEOUT.md` and returns success after rewriting the file.

Initial disposition: candidate P2. Likely unique contract/runtime drift.

### F05 P2: `review record` docs still advertise `REPLAN_REQUIRED`, but the threshold path succeeds with `replan_triggered: true`

Reported by R01.

Evidence:

- CLI reference says threshold crossing errors with `REPLAN_REQUIRED`.
- Implementation increments the dirty counter, sets `replan.triggered`, and still emits a success envelope.

Initial disposition: candidate P2 docs/contract drift.

### F06 P2: `status` reference still overstates the Ralph fallback and is wrong for an empty `PLANS/` tree

Reported by R01.

Evidence:

- Docs still imply bare `status` emits the graceful fallback whenever no mission resolves.
- Runtime now errors with `MISSION_NOT_FOUND` when `PLANS/` exists but has zero missions.

Initial disposition: candidate P2. Likely docs follow-on to round-12 status behavior changes.

### F07 P2: Proof receipts can escape `PLANS/<mission>` through a symlinked `PROOF.md`

Reported by R02.

Evidence:

- `task finish`, `close check`, and `review packet` only check `is_file()` on proofs.
- Repro used `specs/T1/PROOF.md` as a symlink to a file outside the mission tree; task finish and terminal close still succeeded.

Initial disposition: candidate P2. Looks unique.

### F08 P1: `close complete` still publishes `CLOSEOUT.md` before the close mutation commits

Reported by R02 and R16.

Evidence:

- `close complete` writes `CLOSEOUT.md` in the `precommit` callback before `append_event`/state persist.
- Repros using unwritable `EVENTS.jsonl` produced `PARSE_ERROR`, left `STATE.json` non-terminal, and still created `CLOSEOUT.md`.

Initial disposition: likely merge target into round-12 F14 family.

### F09 P1: Dirty planned reviews still advertise rerunning the review instead of repair

Reported by R05 and R11.

Evidence:

- `status` says repair is next while `task next`, `plan waves`, and `plan graph` advertise the review as runnable.
- `review start` then fails with `REVIEW_FINDINGS_BLOCK`.

Initial disposition: likely merge target into round-11 F09 / round-12 F08 family.

### F10 P1: Replan-triggered missions still leak executable work through `status`, `plan waves`, and `plan graph`

Reported by R05.

Evidence:

- `task next` correctly returns `replan`.
- `status.ready_tasks`, `plan waves.current_ready_wave`, and `plan graph` still project stale ready work.

Initial disposition: likely merge target into round-11 F10 family.

### F11 P2: `plan waves` collapses live tasks with superseded dependencies into earlier waves instead of failing closed

Reported by R05.

Evidence:

- With `T2 -> T1` and `T1` superseded, `plan waves` still emits `T2` in `W1` while `plan graph`, `status`, and `task next` all correctly block it.
- `plan check` rejects the same shape in valid lock-time flows, but `plan waves` silently drops the superseded dep.

Initial disposition: candidate P2, probably merge into earlier superseded-DAG projection family.

### F12 P1: Restarted planned review boundaries still accept stale results from the previous round as current

Reported by R06, R07, and R10.

Evidence:

- After a second `review start`, replaying an old findings file still returns `category: accepted_current` and mutates the dirty counter.
- This is caused by the classifier treating a restarted pending record as current rather than fenced.

Initial disposition: likely merge target into round-11 F19 family.

### F13 P2: Late review results arriving during the unlocked replan window are rejected as `PLAN_INVALID` instead of being classified/audited as stale

Reported by R06, R07, and R10.

Evidence:

- Late `review record` after `replan record` still hits the plan-lock guard first.
- No `review.stale` or other audit-only event is appended.

Initial disposition: likely merge target into round-10 F14 / round-11 F15 family.

### F14 P2: `task start` can reopen a target that has already been repaired and is awaiting the advertised re-review

Reported by R06.

Evidence:

- After repair finish, `status` says `run_review`, but another `task start` for the same target still succeeds.
- Task state ends up with `status: in_progress` while retaining the earlier `finished_at`.

Initial disposition: likely merge target into round-10 F11 family.

### F15 P1: Superseded dirty reviews remain blocking truth after replan and relock, even when the rebuilt DAG is complete

Reported by R07.

Evidence:

- Rebuilt mission completed replacement tasks, but `status` and `close check` still blocked on the old dirty review.
- The old review task/targets were no longer in the current locked DAG.

Initial disposition: likely merge target into round-11 F14 family.

### F16 P2: Review dependency readiness still treats any `AwaitingReview` dependency as satisfied, even when that dependency is not one of the review’s targets

Reported by R07.

Evidence:

- A review task depending on both `T2` and `T4` but targeting only `T2` was considered runnable while `T4` was merely `AwaitingReview`.
- `review start` succeeded before `T4` had passed its own review.

Initial disposition: likely merge target into round-11 merged F27 family.

### F17 P1: A passed mission-close review survives replan and blesses new work without a fresh terminal review

Reported by R08, R09, and R10.

Evidence:

- `replan record` and relock do not retire `close.review_state = passed`.
- After new DAG work is completed, `status` and `close check` immediately return close-ready without a new mission-close review.

Initial disposition: likely merge target into round-12 F21 / round-11 F20 family.

### F18 P2: `replan record` mutates terminal missions and leaves publicly contradictory terminal state

Reported by R08.

Evidence:

- `replan record` succeeds on a terminal mission, sets `phase: plan`, `plan.locked: false`, and `replan.triggered: true`, while preserving `close.terminal_at`.
- `status` then reports a terminal verdict with replan-required plan-phase fields.

Initial disposition: candidate P2. Looks unique.

### F19 P2: The published `status` contract still claims `close_ready` is just `verdict == mission_close_review_passed`

Reported by R11.

Evidence:

- README and `docs/cli-contract-schemas.md` still describe the old derivation.
- Runtime now uses `ReadinessReport.ready`, which additionally requires zero blockers.

Initial disposition: likely docs follow-on to round-12 F22.

### F20 P2: `make verify-contract` is not a reliable verification gate because the default `cargo test` run flakes on a missing `target/debug/codex1`

Reported by R13.

Evidence:

- Reviewer reproduced multiple `cargo test` / `make verify-contract` failures involving `assert_cmd::cargo_bin` missing `target/debug/codex1`.
- The same tests pass in isolation, indicating a flaky verification path rather than a deterministic failure.

Initial disposition: candidate P2. Looks unique but needs finding-review scrutiny because it conflicts with main-thread local verification.

### F21 P2: The published custom-`INSTALL_DIR` verification recipe can fail outright or verify the wrong binary from `/tmp`

Reported by R13.

Evidence:

- Docs say custom install locations are supported, but the published PATH recipe still hard-codes `$HOME/.local/bin`.
- Repro showed the recipe either failing to find the custom install or resolving a stub from the default install dir instead.

Initial disposition: candidate P2. Looks unique docs/user-flow drift.

### F22 P2: `replan record` docs still promise a scalar `supersedes`, but the CLI emits an array

Reported by R15.

Evidence:

- CLI reference still documents `"supersedes":"T4"`.
- Implementation and tests emit/expect an array, and live repro returned `["T1"]`.

Initial disposition: candidate P2 docs drift.

### F23 P2: `status` / `close check` still announce `close_ready: true` when `close complete` is guaranteed to fail on directory permissions

Reported by R16.

Evidence:

- With mission dir permissions set read-only, `close check` and `status` still project close-ready.
- `close complete` then fails with a permission-denied temp-file write.

Initial disposition: likely merge target into round-12 F22 family.

### F24 P2: `plan scaffold` still records a scaffold event before confirming `PLAN.yaml` can actually be published

Reported by R16.

Evidence:

- Permission-denied repro left `EVENTS.jsonl` with a `plan.scaffold` event while `STATE.json` remained unchanged and `PLAN.yaml` stayed old.

Initial disposition: likely merge target into round-12 F15 family.
