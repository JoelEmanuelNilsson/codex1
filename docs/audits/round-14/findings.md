# Round 14 Heavy Review Findings

Date: 2026-04-21

Baseline under review: `d88ecda3f1098a4cc8eb4bff2a0e9368da762d49`

This round deployed 16 read-only reviewer lanes using `gpt-5.4` with high reasoning. Every reviewer was instructed to read `README.md` and every markdown file under `docs/**/*.md`, pay special attention to `docs/codex1-rebuild-handoff/` as the primary intended-state source, use prior audit decisions as supporting context, mutate no repo mission state, and report only verified P0/P1/P2 findings after trying to disprove each candidate.

Baseline verification before review:

- `cargo fmt --check` passed.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo test` passed.
- `make verify-contract` passed.

## Summary

- P0: 0
- P1: 15
- P2: 15

Raw reviewer counts before dedupe: 30 findings.

## Reviewer Management Table

| Reviewer ID | Assigned surface | Model | Reasoning | Result | P0 | P1 | P2 | Finding titles | Evidence quality | Duplicate/unique guess | Repro status | Main-thread initial disposition | Recommended next step |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- |
| R01 | CLI contract and envelopes | `gpt-5.4` | high | Findings | 0 | 1 | 2 | Replan-triggered work still executable; `task next` replan docs drift; task status docs drift | High | First is long-open runtime family; latter two look docs-only unique | Reproduced | Candidate P1 + P2s | Status/task/docs shard |
| R02 | Mission resolution and path security | `gpt-5.4` | high | Findings | 0 | 0 | 2 | Symlinked fake mission counted as ambiguous candidate; symlinked `reviews/` poisons closeout history | High | Both look unique | Reproduced | Candidate P2s | Mission-path / closeout shard |
| R03 | Outcome and clarify | `gpt-5.4` | high | Findings | 0 | 0 | 2 | Indented YAML passes `outcome check` but cannot ratify; forbidden workflow-policy fields ratify into `OUTCOME.md` | High | Both look unique | Reproduced | Candidate P2s | Outcome/clarify shard |
| R04 | Plan validation | `gpt-5.4` | high | Findings | 0 | 1 | 1 | Superseded dirty review still blocks rebuilt DAG; unlocked-replan late review returns `PLAN_INVALID` | High | Both match already-open lifecycle families | Reproduced | Candidate merge set | Replan/review shard |
| R05 | DAG, waves, and graph | `gpt-5.4` | high | Findings | 0 | 1 | 1 | Dirty review still advertised as runnable review work; non-target `AwaitingReview` deps satisfy review readiness | High | Both match long-open readiness families | Reproduced | Candidate merge set | DAG/readiness shard |
| R06 | Task lifecycle | `gpt-5.4` | high | Findings | 0 | 2 | 0 | Locked-plan execution ignores `plan.hash`; stale dirty review after replan still leaves contradictory runnable work | High | First looks unique; second merges stale-review families | Reproduced | Candidate P1 + merge | Task/plan-hash shard |
| R07 | Review lifecycle | `gpt-5.4` | high | Findings | 0 | 1 | 1 | Superseded dirty reviews still block after replan; unlocked-replan late review returns `PLAN_INVALID` | High | Both match already-open review-boundary families | Reproduced | Candidate merge set | Review-boundary shard |
| R08 | Replan lifecycle | `gpt-5.4` | high | Findings | 0 | 2 | 1 | Replan-triggered work still startable; superseded dirty review survives replan; unlocked-replan late review returns `PLAN_INVALID` | High | All three match already-open lifecycle families | Reproduced | Candidate merge set | Replan shard |
| R09 | Close lifecycle | `gpt-5.4` | high | Findings | 0 | 1 | 0 | Mission-close review still has no round identity | High | Matches old mission-close boundary family | Reproduced | Candidate merge set | Mission-close boundary shard |
| R10 | State persistence and concurrency | `gpt-5.4` | high | Findings | 0 | 2 | 0 | Locked-plan hash bypass on execution/readiness paths; close artifacts still publish before commit under post-precommit failure | High | First likely unique; second matches old publication family | Reproduced | Candidate P1 + merge | Transaction/plan-hash shard |
| R11 | Status and Ralph | `gpt-5.4` | high | Findings | 0 | 1 | 1 | Replan-triggered missions still leak executable work through `status`; Ralph fails open on explicit selector errors | High | First matches old replan family; second looks unique | Reproduced | Candidate P1 + P2 | Status/Ralph shard |
| R12 | Loop commands and orchestration skills | `gpt-5.4` | high | Findings | 0 | 0 | 2 | `$autopilot` skips mandatory `close check`; `$review-loop` mission-close packet requires undefined preview artifacts | High | Both look unique docs/skill issues | Reproduced | Candidate P2s | Skills/docs shard |
| R13 | Install and Makefile UX | `gpt-5.4` | high | Findings | 0 | 0 | 1 | `INSTALL_DIR` with spaces is broken and verification can pass against wrong path | High | Old install family still live | Reproduced | Candidate P2 | Install/Makefile shard |
| R14 | Test adequacy | `gpt-5.4` | high | Findings | 0 | 2 | 0 | Superseded dirty review still bricks mission post-replan; restarted review boundary accepts stale prior findings as current | High | First matches old family; second matches old boundary family | Reproduced | Candidate merge set | Review/replan coverage shard |
| R15 | Docs and handoff cross-check | `gpt-5.4` | high | Findings | 0 | 1 | 1 | Ralph ambiguous-mission docs drift; docs still describe shipped surfaces as future Phase-B / `NOT_IMPLEMENTED` | High | First likely unique docs/runtime drift; second unique docs drift | Reproduced | Candidate P1 + P2 | Docs/handoff shard |
| R16 | Current diff and regression review | `gpt-5.4` | high | `NONE` | 0 | 0 | 0 | None | High | No verified issues | N/A | No finding | No action |

## Deduplicated Raw Finding Index

These are raw candidates for finding review. Severity here reflects the reporting reviewer severity, not the final accepted severity.

### F01 P1: Replan-triggered missions still leak executable work and remain startable even while readiness says replan is required

Reported by R01, R08, and R11.

Evidence:

- `state::readiness::derive_verdict` still treats `replan.triggered` as `blocked`.
- `status` still publishes `ready_tasks`.
- `task start` still succeeds for ready tasks while `replan.triggered=true`.

Initial disposition: likely merge target into the long-open round-11 F10 family unless finding review decides the latest evidence warrants a fresh standalone item.

### F02 P2: `task next` docs still advertise `REPLAN_REQUIRED` as an error even though the runtime returns a success envelope with `next.kind = "replan"`

Reported by R01.

Evidence:

- `docs/cli-reference.md` still lists `REPLAN_REQUIRED` in the `task next` error set.
- Runtime emits `ok:true` with `data.next.kind = "replan"` and exit code `0`.

Initial disposition: candidate P2 docs drift.

### F03 P2: Task lifecycle docs still publish PascalCase statuses (`InProgress`, `Complete`) that the runtime no longer emits

Reported by R01.

Evidence:

- `docs/cli-reference.md` examples use PascalCase task statuses.
- Runtime and tests use snake_case (`in_progress`, `complete`).

Initial disposition: candidate P2 docs drift.

### F04 P2: Bare mission discovery counts symlinked non-missions as real candidates, so a single valid mission can become spuriously ambiguous

Reported by R02.

Evidence:

- Discovery counts `PLANS/*` entries using `is_dir()` plus `STATE.json.is_file()`, which follow symlinks.
- Later path validation rejects the same symlinked mission root as invalid.
- Bare `status` can fail ambiguous while explicit `--mission real` succeeds.

Initial disposition: candidate P2.

### F05 P2: `close complete` trusts a symlinked `reviews/` directory when reconstructing mission-close history, so external files can poison `CLOSEOUT.md`

Reported by R02.

Evidence:

- `mission_close_dirty_rounds` counts `reviews/mission-close-*.md` via raw `read_dir`.
- No containment check rejects a symlinked `reviews/` directory.
- External review files can force `CLOSEOUT.md` to claim dirty mission-close history the mission never recorded.

Initial disposition: candidate P2.

### F06 P2: `outcome ratify` rejects valid indented YAML frontmatter even though `outcome check` accepts it as ratifiable

Reported by R03.

Evidence:

- Validation accepts valid YAML mappings regardless of indentation.
- `rewrite_status_to_ratified` still assumes top-level keys have no leading whitespace.
- Valid frontmatter with `  status: draft` passes `outcome check` but fails `outcome ratify`.

Initial disposition: candidate P2.

### F07 P2: Forbidden workflow-policy fields such as `approval_boundaries` and `autonomy` can still be ratified into `OUTCOME.md`

Reported by R03.

Evidence:

- Handoff and clarify skill explicitly forbid these fields in mission destination truth.
- Validator never rejects them.
- `outcome ratify` rewrites status and preserves those forbidden keys.

Initial disposition: candidate P2.

### F08 P1: Superseded dirty planned-review truth still survives replan/relock and blocks the rebuilt DAG

Reported by R04, R07, R08, and R14.

Evidence:

- `replan record` supersedes tasks and clears counters but does not retire stale dirty reviews.
- `plan check` relock clears the replan gate without retiring that stale review truth.
- `status` / `close check` still block on the old dirty review even after replacement tasks complete.

Initial disposition: likely merge target into the long-open round-11 F14 family.

### F09 P2: Late review results during the unlocked replan window are still rejected as `PLAN_INVALID` instead of being stale-audited as `stale_superseded`

Reported by R04, R07, and R08.

Evidence:

- `review record` enforces `require_plan_locked` before stale/superseded classification.
- After `replan record` unlocks the plan, the late stale-audit path is unreachable.
- Repro shows `PLAN_INVALID` and no stale audit event in `EVENTS.jsonl`.

Initial disposition: likely merge target into the long-open round-10 F14 / round-11 merged F15 family.

### F10 P1: Dirty accepted-current reviews are still advertised as runnable review work before repair is complete

Reported by R05.

Evidence:

- `status.next_action` says repair is required.
- `task next`, `status.review_required`, `plan waves`, and `plan graph` still advertise the review as ready/runnable.
- `review start` then fails with `REVIEW_FINDINGS_BLOCK`.

Initial disposition: likely merge target into the long-open round-11 F09 family.

### F11 P2: Review dependency readiness still treats non-target `AwaitingReview` dependencies as satisfied

Reported by R05.

Evidence:

- Review task readiness allows any non-target dep in `AwaitingReview`.
- Review start/record can succeed even while a declared prerequisite task has not yet passed its own review.

Initial disposition: likely merge target into the long-open round-10 F16 / round-11 merged F27 family.

### F12 P1: Locked-plan execution/readiness surfaces ignore `state.plan.hash`, so post-lock `PLAN.yaml` edits can change live work without any replan or relock

Reported by R06 and R10.

Evidence:

- `plan check` rejects changed locked plans via `plan.hash`.
- Execution/readiness surfaces still reload live `PLAN.yaml` directly and never compare it to the locked hash.
- Post-lock edits can inject new tasks or change readiness immediately, and `task start` can mutate state for tasks not in `state.plan.task_ids`.

Initial disposition: candidate P1. Looks unique and high-signal.

### F13 P1: Restarting a planned review still does not fence off stale findings from the previous review round

Reported by R14.

Evidence:

- A second `review start` overwrites the old record with a fresh pending boundary.
- Replaying an old findings file after restart still yields `accepted_current`.
- The stale replay can increment the dirty counter toward replan.

Initial disposition: likely merge target into the long-open round-11 F19 family.

### F14 P1: Mission-close review still has no round identity, so a stale clean can satisfy the terminal gate

Reported by R09.

Evidence:

- Mission-close review state has no boundary token comparable to planned reviews’ `boundary_revision`.
- A dirty mission-close result followed by an unrelated state change and stale clean can still flip `close.review_state` to `passed`.
- Planned review path correctly fences the comparable stale result.

Initial disposition: likely merge target into the long-open round-11 F20 family.

### F15 P1: `close complete` and dirty `close record-review` still publish canonical artifacts before the event/state commit succeeds

Reported by R10.

Evidence:

- `CLOSEOUT.md` and `reviews/mission-close-*.md` are written before `append_event` / final state persistence.
- Repros using post-precommit `EVENTS.jsonl` failure produce `PARSE_ERROR`, unchanged state, and already-published canonical artifacts.

Initial disposition: likely merge target into the long-open round-12 F14 / round-13 F08 publication family.

### F16 P2: Ralph hook still fails open on explicit selector errors (`CODEX1_MISSION`, `CODEX1_REPO_ROOT`) instead of honoring the status failure

Reported by R11.

Evidence:

- Explicit bad selectors make `codex1 status` return `MISSION_NOT_FOUND`.
- Hook only special-cases ambiguous bare discovery.
- For other error envelopes it cannot parse `stop.allow`, prints an allow-stop warning, and exits `0`.

Initial disposition: candidate P2.

### F17 P2: `$autopilot`’s published flow still skips the required `close check` gate before `close complete`

Reported by R12.

Evidence:

- Handoff/autopilot flow requires mission-close review, then `close check`, then `close complete`.
- Public autopilot skill dispatch goes directly from `next_action.kind == close` to `codex1 close complete`.
- Same skill separately says not to do that unless the most recent `close check` returned `ready:true`, but never runs the command.

Initial disposition: candidate P2.

### F18 P2: `$review-loop`’s mission-close workflow still requires undefined `CLOSEOUT-preview` / proof-index artifacts that the public CLI does not provide

Reported by R12.

Evidence:

- Review-loop skill and reviewer profiles require a `CLOSEOUT-preview` plus proof index in the mission-close reviewer packet.
- Public CLI surface has no close-preview or mission-close-packet command.
- Actual `closeout.rs` only renders final `CLOSEOUT.md` at `close complete`.

Initial disposition: candidate P2.

### F19 P2: `INSTALL_DIR` values with spaces are still broken, and `verify-installed` can falsely pass against the wrong path

Reported by R13.

Evidence:

- `Makefile` uses `$(abspath $(INSTALL_DIR))`, which splits space-containing paths.
- `install-local` and `verify-installed` can claim success for a malformed path that is not the user-requested install directory.

Initial disposition: candidate P2.

### F20 P1: Ralph’s published stop contract is still false for ambiguous multi-mission repos

Reported by R15.

Evidence:

- Public docs say Ralph blocks only when a resolved status reports `stop.allow == false`.
- Shipped hook still blocks ambiguous bare multi-mission discovery before any resolved mission / `stop.allow` value exists.
- Repro shows two inactive missions still cause the hook to exit `2`.

Initial disposition: candidate P1 docs/runtime drift.

### F21 P2: Top-level docs still describe shipped hook/skill surfaces as future “Phase B” work and tell operators to expect impossible `NOT_IMPLEMENTED` failures

Reported by R15.

Evidence:

- README/install/CLI docs still contain “once that unit lands” and “Phase B” wording for already-shipped hook/skill surfaces.
- Repo already ships the hook docs/script and all six skills.
- No live command path constructs `CliError::NotImplemented`.

Initial disposition: candidate P2 docs drift.

## Clean Lanes

- R16 `Current diff and regression review` returned `NONE` after disproving an apparent false alarm caused by concurrent local build/test races during review.
