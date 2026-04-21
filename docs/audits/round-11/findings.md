# Round 11 Heavy Review Findings

Date: 2026-04-21

This round deployed 16 read-only reviewer lanes using `gpt-5.4` with high reasoning. Every reviewer was instructed to read `README.md` and every markdown file under `docs/**/*.md`, pay special attention to `docs/codex1-rebuild-handoff/` as the primary intended-state source, use prior audit decisions as supporting context, mutate no repo mission state, and report only verified P0/P1/P2 findings.

Baseline verification before review:

- `cargo fmt --check` passed.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo test` passed.

## Summary

- P0: 0
- P1: 9
- P2: 21

Raw reviewer counts before dedupe: 30 findings.

## Reviewer Management Table

| Reviewer ID | Assigned surface | Model | Reasoning | Execution wave | Result | P0 | P1 | P2 | Finding titles | Evidence quality | Duplicate/unique guess | Repro status | Main-thread initial disposition | Recommended next step |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- |
| R01 | CLI contract and envelopes | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 2 | `task next` mission-close/terminal handoff drift; stale review start/status envelope docs | High | First likely overlaps R12 on `task next`; docs item unique | Reproduced | Candidate P2s | Shard with orchestration/docs reviewers |
| R02 | Mission resolution and path security | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 2 | Symlinked `PLAN.yaml` still trusted by graph/waves; symlinked `OUTCOME.md` still trusted by packet/closeout readers | High | Likely incomplete-repair follow-ons to round-10 F15 | Reproduced | Candidate P2s | Shard with containment/read-side reviewers |
| R03 | Outcome and clarify | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 2 | Empty required OUTCOME list fields still ratify; OUTCOME `mission_id` mismatch ignored | High | Both look unique | Reproduced | Candidate P2s | Shard with validator/contract reviewers |
| R04 | Plan validation | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 1 | Replan can reuse historical task IDs; `plan check` accepts stored `waves:` truth | High | Both look unique | Reproduced | Candidate P1/P2 | Shard with DAG/plan contract reviewers |
| R05 | DAG, waves, and graph | `gpt-5.4` | high | 1 | Findings | 0 | 2 | 0 | Dirty planned reviews advertised as ready review work; triggered replan still leaks executable work | High | First adjacent to round-10 F11; second likely unique regression | Reproduced | Candidate P1s | Shard with task/status/replan reviewers |
| R06 | Task lifecycle | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 2 | Repaired dirty-review target stays blocked; `task start` can reopen repaired target; fresh mission `task next` says `plan` not `clarify` | High | First overlaps R05/F11 family; others likely unique | Reproduced | Candidate P1/P2s | Shard with lifecycle/orchestration reviewers |
| R07 | Review lifecycle | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 0 | Superseded dirty planned reviews remain current blockers after replan | High | Likely overlaps R08 replan review-truth issue | Reproduced | Candidate P1 | Shard with replan/review truth reviewers |
| R08 | Replan lifecycle | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 1 | Dirty-triggered replans never clear superseded dirty review; post-replan review records fail as `PLAN_INVALID` instead of stale audit | High | First overlaps R07; second overlaps R14 coverage note | Reproduced | Candidate P1/P2 | Shard with review/replan reviewers |
| R09 | Close lifecycle | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 3 | Wrong `close record-review` error contract; terminal mission-close late results dropped; `CLOSEOUT.md` falsely says clean on first round | High | All appear unique | Reproduced | Candidate P2s | Shard with close/state reviewers |
| R10 | State persistence and concurrency | `gpt-5.4` | high | 1 | Findings | 0 | 2 | 0 | Planned review restart does not fence old results; mission-close review lacks round identity | High | Both likely unique, though adjacent to prior review-boundary fixes | Reproduced | Candidate P1s | Shard with review/close concurrency reviewers |
| R11 | Status and Ralph | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 2 | `status` swallows explicit `--repo-root` discovery failures; status docs still advertise old ambiguity behavior | High | Runtime item likely unique; docs item likely unique | Reproduced | Candidate P2s | Shard with contract/docs reviewers |
| R12 | Loop commands and orchestration skills | `gpt-5.4` | high | 1 | Findings | 0 | 1 | 1 | `$autopilot` always asks user before close; `task next` never advances past mission-close review | High | `task next` item duplicates R01; autopilot item unique | Reproduced / doc-verified | Candidate P1/P2 | Shard with CLI/orchestration reviewers |
| R13 | Install and Makefile UX | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 2 | `verify-installed` still misses PATH usability from `/tmp`; `doctor` trusts non-executable PATH shadow | High | Both likely unique though adjacent to prior install fixes | Reproduced | Candidate P2s | Shard with install/docs reviewers |
| R14 | Test adequacy | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 1 | Missing regression left accepted stale-after-replan review bug live | High | Duplicate of R08 runtime issue, but useful coverage evidence | Reproduced | Merge into runtime bug, probably non-standalone | Use as test requirement if runtime item accepted |
| R15 | Docs and handoff cross-check | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 1 | `review packet` docs promise proof paths the CLI does not emit | High | Unique docs/packet contract drift | Reproduced | Candidate P2 | Shard with docs/contract reviewers |
| R16 | Current diff and regression review | `gpt-5.4` | high | 1 | Findings | 0 | 0 | 1 | Review deps still allow non-target `AwaitingReview` dependencies to pass | High | Likely continuation of round-10 F16 repair area | Reproduced | Candidate P2 | Shard with review/DAG reviewers |

## Deduplicated Raw Finding Index

These are raw candidates for finding review. Severity here reflects the reporting reviewer severity, not the final accepted severity.

### F01 P2: `task next` keeps advertising `mission_close_review` after close is ready or the mission is terminal

Reported by R01 and R12.

Evidence:

- [crates/codex1/src/cli/task/next.rs](/Users/joel/codex1/crates/codex1/src/cli/task/next.rs:58) returns `mission_close_review` whenever `all_tasks_terminal(...)` is true, without checking `close.review_state` or `close.terminal_at`.
- In `/tmp/codex1-r11-tasknext6.IDtI0N`, after `close record-review --clean`, `status` reported `next_action.kind: "close"` while `task next` still returned `kind: "mission_close_review"`.
- In `/tmp/codex1-r11-tasknextterm.DxsQ2q`, after `close complete`, `status` reported `kind: "closed"` while `task next` still returned `kind: "mission_close_review"`.

Initial disposition: candidate P2. Likely accepted unless finding review decides this surface is intentionally narrower than advertised.

### F02 P2: Review docs are stale for implemented `review start` / `review status` envelopes

Reported by R01.

Evidence:

- [docs/cli-reference.md](/Users/joel/codex1/docs/cli-reference.md:253) documents old `review start` and `review status` shapes.
- [crates/codex1/src/cli/review/start.rs](/Users/joel/codex1/crates/codex1/src/cli/review/start.rs:125) and [crates/codex1/src/cli/review/status_cmd.rs](/Users/joel/codex1/crates/codex1/src/cli/review/status_cmd.rs:52) emit materially different JSON fields.
- Live repro in `/tmp/codex1-r11-reviewstart.vpuY9C` confirmed the docs do not match the installed binary.

Initial disposition: candidate P2 docs/contract drift.

### F03 P2: `plan graph` / `plan waves` still trust a symlinked `PLAN.yaml` outside the mission root

Reported by R02.

Evidence:

- [crates/codex1/src/cli/plan/waves.rs](/Users/joel/codex1/crates/codex1/src/cli/plan/waves.rs:45) and [crates/codex1/src/cli/plan/graph.rs](/Users/joel/codex1/crates/codex1/src/cli/plan/graph.rs:45) call `read_to_string(plan_path)` after `is_file()` and do not use the round-10 safe-read helper.
- Repro in `/tmp/codex1-plan-symlink.IeNxNC` returned an external node `X1` through `plan graph --format json`.

Initial disposition: candidate P2. Looks like an incomplete repair of round-10 F15.

### F04 P2: Packet and closeout paths still trust a symlinked `OUTCOME.md` outside the mission root

Reported by R02.

Evidence:

- [crates/codex1/src/cli/task/worker_packet.rs](/Users/joel/codex1/crates/codex1/src/cli/task/worker_packet.rs:34), [crates/codex1/src/cli/review/packet.rs](/Users/joel/codex1/crates/codex1/src/cli/review/packet.rs:58), and [crates/codex1/src/cli/close/closeout.rs](/Users/joel/codex1/crates/codex1/src/cli/close/closeout.rs:34) read `OUTCOME.md` directly.
- Repros in `/tmp/codex1-outcome-leak.1W5Akv` and `/tmp/codex1-close-poison.WxZ55m` leaked/persisted external content into packets and `CLOSEOUT.md`.

Initial disposition: candidate P2. Likely accepted with F03 under read-side containment hardening, or kept distinct by file surface.

### F05 P2: Empty required OUTCOME list fields still pass `outcome check` / `outcome ratify`

Reported by R03.

Evidence:

- [crates/codex1/src/cli/outcome/validate.rs](/Users/joel/codex1/crates/codex1/src/cli/outcome/validate.rs:94) uses `check_list_present` for several required list fields, which allows `[]`.
- Repro in `/tmp/codex1-outcome-empty-Tl0R2l` ratified an OUTCOME with empty `non_goals`, `constraints`, `quality_bar`, `proof_expectations`, `review_expectations`, and `known_risks`.

Initial disposition: candidate P2. Distinct from prior missing-field findings.

### F06 P2: OUTCOME `mission_id` is not checked against the actual mission directory

Reported by R03.

Evidence:

- [crates/codex1/src/cli/outcome/validate.rs](/Users/joel/codex1/crates/codex1/src/cli/outcome/validate.rs:60) only checks `mission_id` presence/non-empty.
- Repro with `PLANS/demo/OUTCOME.md` containing `mission_id: other-mission` still passed both `outcome check` and `outcome ratify`.

Initial disposition: candidate P2.

### F07 P1: `plan check` relocks replans that reuse historical task IDs, so new work inherits stale completion state

Reported by R04.

Evidence:

- [docs/codex1-rebuild-handoff/03-planning-artifacts.md](/Users/joel/codex1/docs/codex1-rebuild-handoff/03-planning-artifacts.md:357) says replans append new task IDs rather than reuse old ones.
- [crates/codex1/src/cli/plan/check.rs](/Users/joel/codex1/crates/codex1/src/cli/plan/check.rs:444) validates uniqueness only within the current plan, not against historical `STATE.json`.
- Repro showed a new `T1` being silently treated as already complete after replan and relock.

Initial disposition: candidate P1. Strong state aliasing bug.

### F08 P2: `plan check` accepts forbidden stored `waves:` truth in `PLAN.yaml`

Reported by R04.

Evidence:

- [docs/codex1-rebuild-handoff/03-planning-artifacts.md](/Users/joel/codex1/docs/codex1-rebuild-handoff/03-planning-artifacts.md:402) forbids storing waves as editable truth.
- [crates/codex1/src/cli/plan/parsed.rs](/Users/joel/codex1/crates/codex1/src/cli/plan/parsed.rs:10) ignores unknown top-level keys, so `waves:` is silently accepted.
- Repro locked a plan containing `waves: [{ wave_id: W999, tasks: [T1] }]`; `plan waves` then ignored it and derived `W1`.

Initial disposition: candidate P2.

### F09 P1: Dirty planned reviews are still advertised as ready review work even though repair is the only executable next step

Reported by R05.

Evidence:

- In `/tmp/codex1-r05-dirty.lHh5L7`, `status` advertised `repair` while also surfacing `review_required`, `task next` returned `run_review`, `plan waves` advanced to the review wave, and `plan graph --format json` marked the review node `ready`.
- `review start T3` then failed with `REVIEW_FINDINGS_BLOCK`, proving the advertised action was not actually executable.

Initial disposition: candidate P1. Related to the dirty-review repair loop, but distinct from the post-repair routing issue already fixed/accepted in prior rounds.

### F10 P1: A triggered replan does not actually block stale-plan execution

Reported by R05.

Evidence:

- In `/tmp/codex1-r05-replan.75BwRH`, `status` returned `verdict: "blocked"` and `next_action.kind: "replan"` while still exposing `ready_tasks: ["T1"]`.
- `plan waves` still returned `current_ready_wave: W1`.
- `task start T1` still succeeded while `replan.triggered` was true.

Initial disposition: candidate P1. Strong runtime contradiction in the replan gate.

### F11 P1: Repaired dirty-review targets stay `verdict: "blocked"` even when the next action is the ready re-review

Reported by R06.

Evidence:

- Repro in `/tmp/codex1-r06-72rLaL`: after repairing `T2`, `status` returned `verdict: "blocked"` with `next_action.kind: "run_review"` and `review_required` for `T5`.
- `dirty_repair_targets` had already dropped `T2`, but `derive_verdict` still blocked on the old dirty review.
- The autopilot state machine does not treat `blocked + run_review` as a normal flow.

Initial disposition: candidate P1. Likely related to round-10 F11 but not identical; this is the blocked verdict after the handoff switches to review.

### F12 P2: `task start` can reopen a target that has already been repaired and is waiting for re-review

Reported by R06.

Evidence:

- In the same `/tmp/codex1-r06-72rLaL` mission, after repair and after `status` advertised `run_review`, a second `task start T2` still succeeded.
- Final state showed `T2.status = "in_progress"` while preserving the old `finished_at`, indicating an inconsistent reopened repair cycle.

Initial disposition: candidate P2.

### F13 P2: `task next` tells a fresh mission to plan instead of clarify

Reported by R06.

Evidence:

- Fresh mission repro in `/tmp/codex1-r06-fresh-lT9DIn`: `status` correctly returned `next_action.kind: "clarify"` with `outcome_ratified: false`.
- `task next` instead returned `kind: "plan"` because it checks `!plan.locked` but not `!outcome.ratified`.

Initial disposition: candidate P2.

### F14 P1: Superseded dirty planned reviews remain current blockers after replan and can strand the mission

Reported by R07 and R08.

Evidence:

- [crates/codex1/src/state/readiness.rs](/Users/joel/codex1/crates/codex1/src/state/readiness.rs:97) blocks on any stored accepted-current dirty review without checking that its task/targets remain live after replan.
- Repros in `/tmp/codex1-r07b-YUcTNB` and `/tmp/codex1-r08b.FRX2fM` showed superseded review `T3` still blocking `status` / `close check` after relock and replacement work/review.

Initial disposition: candidate P1. Merge R07 and the first half of R08 here.

### F15 P2: Review results that arrive after `replan record` still fail as `PLAN_INVALID` instead of being audited as stale/superseded

Reported by R08 and reinforced by R14.

Evidence:

- Repro in `/tmp/codex1-r08d.d00D4L`: after `replan record --supersedes ...`, `review record T3 --findings-file ...` returned `PLAN_INVALID` instead of a stale category/audit path.
- [crates/codex1/src/cli/review/record.rs](/Users/joel/codex1/crates/codex1/src/cli/review/record.rs:73) still checks `require_plan_locked(&peek)?` before the stale path when the plan is unlocked.
- R14 confirmed there is no regression test for this accepted round-10 case after replan unlock.

Initial disposition: candidate P2. Likely accepted; R14 should probably merge as supporting test evidence, not a standalone finding.

### F16 P2: `close record-review --findings-file` returns the wrong public error contract

Reported by R09.

Evidence:

- [crates/codex1/src/cli/close/record_review.rs](/Users/joel/codex1/crates/codex1/src/cli/close/record_review.rs:163) returns `CliError::ProofMissing` for a missing findings file.
- Live repro returned `code: "PROOF_MISSING"` with a `task finish` hint, even though docs advertise `REVIEW_FINDINGS_BLOCK`.

Initial disposition: candidate P2. Strong contract mismatch.

### F17 P2: Post-terminal mission-close review results are dropped instead of being audited as `contaminated_after_terminal`

Reported by R09.

Evidence:

- [docs/cli-contract-schemas.md](/Users/joel/codex1/docs/cli-contract-schemas.md:155) says contaminated records are appended for audit.
- Repro against a terminal mission returned `CLOSE_NOT_READY` and appended no event.

Initial disposition: candidate P2.

### F18 P2: `CLOSEOUT.md` can falsely claim mission-close review was clean on the first round

Reported by R09.

Evidence:

- [crates/codex1/src/cli/close/closeout.rs](/Users/joel/codex1/crates/codex1/src/cli/close/closeout.rs:94) derives mission-close summary from the dirty counter.
- `close record-review --clean` resets that counter to `0`, so a dirty-then-clean history still renders as “Clean on the first round.”

Initial disposition: candidate P2.

### F19 P1: Restarting a planned review does not fence off late results from the previous boundary

Reported by R10.

Evidence:

- Repro in `/tmp/codex1-review-race2.Lirfxh`: after `review start T5`, dirty findings, repair, and a second `review start T5`, replaying the old dirty result still recorded as `accepted_current`.
- [crates/codex1/src/cli/review/classify.rs](/Users/joel/codex1/crates/codex1/src/cli/review/classify.rs:43) only emits `late_same_boundary` when the existing review is non-pending, so a restarted pending record lets the old result fall through as current.

Initial disposition: candidate P1.

### F20 P1: Mission-close review has no round identity, so a stale clean can still pass the terminal gate

Reported by R10.

Evidence:

- [crates/codex1/src/state/schema.rs](/Users/joel/codex1/crates/codex1/src/state/schema.rs:152) has no mission-close boundary field.
- Repro in `/tmp/codex1-close-race.TfMmzU`: a dirty record followed immediately by a clean record on the open mission-close boundary left `review_state: "passed"` with no way to distinguish stale clean from current-round clean.

Initial disposition: candidate P1. Strong terminal-boundary truth issue.

### F21 P2: `status` swallows explicit `--repo-root` discovery failures into the bare Ralph fallback

Reported by R11.

Evidence:

- [crates/codex1/src/cli/status/mod.rs](/Users/joel/codex1/crates/codex1/src/cli/status/mod.rs:33) only rethrows `MISSION_NOT_FOUND` on explicit `--mission` or ambiguous discovery, not on explicit `--repo-root`.
- Repro with an explicit empty root returned a graceful `no_mission` envelope and let `scripts/ralph-stop-hook.sh` exit `0`.

Initial disposition: candidate P2.

### F22 P2: Public `status` docs still advertise the pre-repair ambiguity behavior

Reported by R11.

Evidence:

- [docs/cli-reference.md](/Users/joel/codex1/docs/cli-reference.md:465) still says no-single-mission resolution emits a graceful fallback and that `MISSION_NOT_FOUND` happens only with explicit `--mission`.
- Current code and [crates/codex1/tests/foundation.rs](/Users/joel/codex1/crates/codex1/tests/foundation.rs:278) assert bare ambiguous discovery now errors.

Initial disposition: candidate P2 docs drift.

### F23 P1: `$autopilot` cannot actually finish autonomously because its close handoff always asks the user again

Reported by R12.

Evidence:

- [.codex/skills/autopilot/SKILL.md](/Users/joel/codex1/.codex/skills/autopilot/SKILL.md:52) says `close` requires user confirmation before `close complete`.
- [.codex/skills/autopilot/references/autopilot-state-machine.md](/Users/joel/codex1/.codex/skills/autopilot/references/autopilot-state-machine.md:68) calls `confirm_with_user(...)` before terminal close.
- The handoff flow in [docs/codex1-rebuild-handoff/01-product-flow.md](/Users/joel/codex1/docs/codex1-rebuild-handoff/01-product-flow.md:78) goes straight from a passing close check to `close complete`.

Initial disposition: candidate P1. Skill-level behavior issue with real flow impact.

### F24 P2: `verify-installed` can still pass even when `codex1` is unusable via `PATH` from `/tmp`

Reported by R13.

Evidence:

- [Makefile](/Users/joel/codex1/Makefile:35) verifies `$(INSTALL_DIR)/codex1` directly, not `command -v codex1`.
- Repro with `PATH=/usr/bin:/bin make verify-installed` passed, but `/tmp` command lookup then failed (`command -v codex1` absent, `codex1 --help` exit `127`).

Initial disposition: candidate P2. Related to earlier install verification work but still open.

### F25 P2: `doctor` reports a non-executable `codex1` file as “on PATH”

Reported by R13.

Evidence:

- [crates/codex1/src/cli/doctor.rs](/Users/joel/codex1/crates/codex1/src/cli/doctor.rs:63) checks `candidate.is_file()` but not executability.
- Repro with a non-executable shadow file caused `doctor` to report it as `codex1_on_path` while shell lookup/execution failed.

Initial disposition: candidate P2.

### F26 P2: `review packet` docs promise proof paths the current CLI never emits

Reported by R15.

Evidence:

- [docs/cli-reference.md](/Users/joel/codex1/docs/cli-reference.md:275) and [docs/cli-contract-schemas.md](/Users/joel/codex1/docs/cli-contract-schemas.md:263) show `proofs:["specs/T2/PROOF.md"]`.
- [crates/codex1/src/cli/review/packet.rs](/Users/joel/codex1/crates/codex1/src/cli/review/packet.rs:107) rewrites proof paths relative to repo root, producing `PLANS/demo/specs/T1/PROOF.md` in a normal repro.

Initial disposition: candidate P2 docs/contract drift.

### F27 P2: Review dependency readiness still allows non-target `AwaitingReview` dependencies

Reported by R16.

Evidence:

- [crates/codex1/src/cli/review/start.rs](/Users/joel/codex1/crates/codex1/src/cli/review/start.rs:149) and [crates/codex1/src/cli/status/next_action.rs](/Users/joel/codex1/crates/codex1/src/cli/status/next_action.rs:252) treat any `AwaitingReview` dependency as ready, not just actual review targets.
- Repro in `/tmp/codex1-reviewdepvalid.LN9zX9` let `T5` review and complete before extra dependency `T4` became review-clean via `T6`.

Initial disposition: candidate P2. Likely a continuation of the round-10 F16 repair area.

## Clean Lanes

No reviewer returned `NONE`; all 16 lanes reported at least one candidate P0/P1/P2. Several are likely duplicates and some doc/test items may be downgraded or merged by finding review.
