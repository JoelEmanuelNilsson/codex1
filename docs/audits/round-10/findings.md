# Round 10 Raw Findings

Baseline under review: `9169e98 round-9 repairs: harden lifecycle concurrency`

Round 10 was started from a clean worktree after round-9 repairs and verification. Sixteen `gpt-5.4` high-reasoning reviewers reviewed the assigned surfaces. This file records raw reviewer output before finding-review deduplication and severity validation.

## Reviewer Summary

| Reviewer | Surface | Raw Result |
| --- | --- | --- |
| R10-01 | CLI contract and envelopes | NONE |
| R10-02 | Mission resolution and path security | 1 P2 |
| R10-03 | Outcome and clarify | 1 P2 |
| R10-04 | Plan validation | NONE |
| R10-05 | DAG, waves, graph | 1 P2 |
| R10-06 | Task lifecycle | 2 P2 |
| R10-07 | Review lifecycle | 2 P2 |
| R10-08 | Replan lifecycle | NONE |
| R10-09 | Close lifecycle | 1 P2 |
| R10-10 | State persistence and concurrency | 1 P1, 1 P2 |
| R10-11 | Status and Ralph | 1 P2 |
| R10-12 | Loop commands and orchestration skills | 1 P1, 1 P2 |
| R10-13 | Install and Makefile UX | NONE |
| R10-14 | Test adequacy | 4 P2 coverage findings |
| R10-15 | Docs/handoff cross-check | 2 P2 docs findings |
| R10-16 | Current diff / regression reviewer | 1 P1, 1 P2 |

## Raw Findings

### F01: `outcome ratify` can fail after ratifying state

Raw severity: P2

Reporter: R10-03

Affected files:
- `crates/codex1/src/cli/outcome/ratify.rs`
- `crates/codex1/src/core/paths.rs`

Evidence:
- `outcome ratify` mutates `STATE.json` before running the write-safety preflight for `OUTCOME.md`.
- If `PLANS/<mission>/OUTCOME.md` is a symlink to a valid external outcome, validation reads the symlink target, `state::mutate` marks `outcome.ratified=true` and advances phase to `plan`, then `ensure_artifact_parent_write_safe` rejects the symlinked artifact.
- The command exits with `ok:false` / `PLAN_INVALID`, but `STATE.json` has `revision:1`, `phase:"plan"`, `outcome.ratified:true`, and an `outcome.ratified` event.

Repro status: reproduced in `/tmp/codex1-outcome-symlink.kSy0Rt`.

Disposition before finding review: likely valid; check duplication with round-9 sidecar/closeout preflight class.

### F02: `task next` collapses ready tasks from different topological waves into one `run_wave`

Raw severity: P2

Reporter: R10-05

Affected files:
- `crates/codex1/src/cli/task/lifecycle.rs`
- `crates/codex1/src/cli/task/next.rs`
- comparison: `crates/codex1/src/cli/status/next_action.rs`
- comparison: `crates/codex1/src/cli/plan/waves.rs`

Evidence:
- `task next` builds a wave from every non-review task whose effective status is `Ready`, regardless of topological depth.
- Repro plan: `T1` root pending, `T2` root complete, `T3 depends_on [T2]` pending. `T1` belongs to W1; `T3` belongs to W2.
- `status` returns `run_task T1`; `plan waves` says current ready wave is W1; `task next` returns `run_wave` with `wave_id:"W1"` and `tasks:["T1","T3"]`.

Repro status: reproduced in `/tmp/codex1-wave.NR40LM`.

Disposition before finding review: likely valid; inspect whether `task next` and status are intended to share the topological wave model.

### F03: `status.stop.allow` now depends on close blockers despite the stop contract allowing `mission_close_review_passed`

Raw severity: P2

Reporter: R10-11

Affected files:
- `crates/codex1/src/cli/status/project.rs`
- `crates/codex1/src/cli/status/mod.rs`
- `crates/codex1/src/cli/close/check.rs`
- `crates/codex1/src/state/readiness.rs`

Evidence:
- Round-9 repair made status use path-aware `close_ready`, but `stop_projection` now forces `allow=false` when loop is active/unpaused, verdict is `mission_close_review_passed`, and proof-aware close readiness has blockers.
- Docs and `readiness::stop_allowed` say `stop.allow` is true when verdict is in `{terminal_complete, mission_close_review_passed, needs_user}` or loop is inactive/paused.
- Repro with active unpaused loop, mission-close review passed, completed task proof file missing: `close check` returns `verdict:"mission_close_review_passed"` and `ready:false`; `status` returns same verdict and `close_ready:false`, but `stop.allow:false`.

Repro status: reproduced with live CLI in a temp mission.

Disposition before finding review: likely valid regression from round-9 F01 repair; repair should keep `close_ready` out of Ralph stop permission while preserving `next_action` close blocking.

### F04: Missing stale `review start` versus malformed `PLAN.yaml` regression test

Raw severity: P2 test adequacy

Reporter: R10-14

Affected files:
- `docs/audits/round-8/repair-plan.md`
- `crates/codex1/tests/review.rs`
- `crates/codex1/tests/foundation.rs`

Evidence:
- Round-8 repair plan required a stale review-start test that wins over malformed `PLAN.yaml`.
- Current review-start stale test flips `plan.locked=false` with a parseable plan. Malformed YAML coverage exists for `plan check`, not `review start`.

Repro status: verified by test search, not by a failing behavior repro.

Disposition before finding review: likely a coverage checklist item, not standalone runtime P2 unless behavior is broken.

### F05: Missing clean-before-stale-dirty mission-close concurrency regression

Raw severity: P2 test adequacy

Reporter: R10-14

Affected files:
- `docs/audits/round-8/repair-plan.md`
- `crates/codex1/tests/close.rs`

Evidence:
- Round-8 required a clean-before-stale-dirty regression for accepted close-review concurrency.
- Existing tests cover serial dirty-then-clean and concurrent dirty writers with `--expect-revision`, but no concurrent dirty-vs-clean test without revision fencing that proves final state cannot be reopened.

Repro status: verified by test search, not by a failing behavior repro.

Disposition before finding review: likely coverage checklist item unless runtime behavior is demonstrably broken.

### F06: Orphan review-task replan close path is still masked by manual state mutation in tests

Raw severity: P2 test adequacy

Reporter: R10-14

Affected files:
- `docs/audits/round-8/repair-plan.md`
- `docs/audits/round-9/findings.md`
- `crates/codex1/tests/e2e_replan_trigger.rs`

Evidence:
- Round-8/round-9 called for a regression where `T2 awaiting_review` plus `T3 review T2` can be replanned without stranding close.
- Current full close-path test still manually writes `state["tasks"]["T3"] = { status:"superseded" }` before replacement work and close, masking whether the CLI handles the orphan review task.

Repro status: verified by test inspection, not by a failing behavior repro.

Disposition before finding review: likely coverage checklist item; may require runtime repro before accepting as P2.

### F07: Status lacks superseded-root live-work dependency regression

Raw severity: P2 test adequacy

Reporter: R10-14

Affected files:
- `docs/audits/round-9/findings.md`
- `crates/codex1/tests/plan_check.rs`
- `crates/codex1/tests/task.rs`
- `crates/codex1/tests/status.rs`

Evidence:
- Round-9 test checklist called for superseded-root live-DAG projection across plan/status/task.
- Current tests cover `plan check` rejection and `task next` not surfacing live work depending on a superseded task.
- Status-side superseded test covers a review target that is superseded, not live work depending on a superseded root.

Repro status: verified by test search, not by a failing behavior repro.

Disposition before finding review: likely coverage checklist item unless `status` behavior is broken.

### F08: README and CLI reference give `task finish --proof` paths that fail

Raw severity: P2 docs

Reporter: R10-15

Affected files:
- `README.md`
- `docs/cli-reference.md`
- implementation: `crates/codex1/src/cli/task/finish.rs`
- correct contract: `docs/cli-contract-schemas.md`

Evidence:
- README and CLI reference tell users to write `PLANS/<mission>/specs/<task>/PROOF.md` and pass that path to `--proof`.
- The schema and implementation resolve relative proof paths against the mission directory, so `--proof PLANS/demo/specs/T1/PROOF.md` becomes `PLANS/demo/PLANS/demo/specs/T1/PROOF.md`.
- Correct invocation is `--proof specs/T1/PROOF.md`.

Repro status: reproduced in `/tmp/codex1-proof-doc.dtGtMz`.

Disposition before finding review: likely valid docs P2 because copy-paste manual flow fails.

### F09: CLI reference and README omit live mission-close and loop subcommands needed for the documented flow

Raw severity: P2 docs

Reporter: R10-15

Affected files:
- `docs/cli-reference.md`
- `README.md`
- implementation: `crates/codex1/src/cli/loop_/mod.rs`
- implementation: `crates/codex1/src/cli/close/mod.rs`
- `docs/codex1-rebuild-handoff/02-cli-contract.md`
- `docs/cli-contract-schemas.md`

Evidence:
- `docs/cli-reference.md` says it has one section per subcommand, but loop omits `activate` and close omits `record-review`.
- README manual flow jumps from planned review tasks to `close check` / `close complete`, without the mission-close `close record-review` step.
- Following README reaches `close check` with `ready:false`, `verdict:"ready_for_mission_close_review"`, and blocker `mission-close review has not started`.

Repro status: reproduced by following README-shaped flow in a temp mission.

Disposition before finding review: likely valid docs P2 if docs are treated as intended user path.

### F10: `close complete` can publish stale or rejected `CLOSEOUT.md` before terminal mutation wins

Raw severity: P2/P1

Reporters: R10-09, R10-10

Affected files:
- `crates/codex1/src/cli/close/complete.rs`
- `crates/codex1/src/cli/close/closeout.rs`

Evidence:
- Round-9 repair prewrites `CLOSEOUT.md` from `terminal_preview` before `state::mutate`.
- Successful close writes `CLOSEOUT.md` with `Final revision` from the pre-mutation state. Repro: command success revision `7`, state revision `7`, `CLOSEOUT.md` says `Final revision: 6`.
- With `--expect-revision 6`, racing `close complete` processes produced one successful terminal mutation and seven `REVISION_CONFLICT`s, but rejected writers still wrote `CLOSEOUT.md`; final closeout timestamp differed from committed `STATE.json.close.terminal_at`.
- Without `--expect-revision`, eight concurrent `close complete` calls all succeeded, revision advanced from 6 to 14, and eight `close.complete` events were appended.

Repro status: reproduced deterministically in temp missions; one repro path reported `/tmp/codex1-close-expect-race.gL1R7f`.

Disposition before finding review: very likely valid. Severity likely P1 because terminal artifact can be authored by rejected stale writer; finding reviewers should confirm severity and merge duplicates.

### F11: Dirty-review repair keeps routing to `repair` after target was repaired and is ready for re-review

Raw severity: P1

Reporters: R10-12, R10-16

Affected files:
- `crates/codex1/src/cli/status/next_action.rs`
- `crates/codex1/src/cli/status/project.rs`
- `crates/codex1/src/state/readiness.rs`
- `crates/codex1/src/cli/task/start.rs`
- `.codex/skills/autopilot/SKILL.md`
- `.codex/skills/execute/SKILL.md`

Evidence:
- `dirty_repair_targets` returns every target of accepted-current dirty reviews without checking whether the target has since been restarted/refinished and is ready for re-review.
- `derive_next_action` prioritizes non-empty `repair_targets` before ready reviews.
- Repro: dirty review of `T2` causes `status.next_action.kind:"repair"`. Run `task start T2`, update proof, `task finish T2`. Status still returns `repair` with `task_ids:["T2"]` while also showing `review_required` for the review task.
- Following the advertised repair action starts `T2` again and hides `review_required`.

Repro status: reproduced in temp missions by R10-12 and R10-16.

Disposition before finding review: likely valid P1 continuation of round-9 F13. Existing regression manually invokes `review start` after repair, bypassing `status`.

### F12: Concurrent loop transitions classify before lock and can resurrect a deactivated loop

Raw severity: P2

Reporter: R10-12

Affected files:
- `crates/codex1/src/cli/loop_/mod.rs`
- `crates/codex1/src/cli/loop_/pause.rs`
- `crates/codex1/src/cli/loop_/resume.rs`
- `crates/codex1/src/cli/loop_/deactivate.rs`

Evidence:
- `run_transition` loads and classifies before entering `state::mutate`.
- The mutation closure assigns the precomputed target loop state without re-running transition logic under the exclusive state lock.
- Repro 1: race `loop pause` against `loop deactivate` from active loop; deactivate succeeds, stale pause then succeeds and leaves `active:true, paused:true, mode:execute`.
- Repro 2: race `loop resume` against `loop deactivate` from active paused loop; deactivate succeeds, stale resume then succeeds and leaves `active:true, paused:false, mode:execute`.

Repro status: reproduced twice in `/tmp`.

Disposition before finding review: likely valid P2; same TOCTOU class as task/replan but loop-specific.

### F13: Concurrent `task finish` can double-commit and overwrite proof truth

Raw severity: P2

Reporters: R10-06, R10-10

Affected files:
- `crates/codex1/src/cli/task/finish.rs`

Evidence:
- `task finish` checks `current_status == InProgress` only on pre-lock snapshot.
- Inside `state::mutate`, closure only re-checks `plan.locked` then unconditionally assigns `status`, `finished_at`, and `proof_path`.
- Repro: race multiple `task finish T1` calls with different proof paths. All succeeded, final revision advanced once per finisher, multiple `task.finished` events were appended, and final `proof_path` was last-writer-wins.

Repro status: reproduced in `/tmp` by two reviewers; one repro path `/tmp/codex1-finish-race.xERaPg`.

Disposition before finding review: likely valid P2; duplicate reporters should merge.

### F14: Review record racing with replan returns `PLAN_INVALID` and drops stale audit event

Raw severity: P2

Reporters: R10-06, R10-16

Affected files:
- `crates/codex1/src/cli/review/record.rs`
- `crates/codex1/src/state/mod.rs`

Evidence:
- `review record` has a pre-lock stale path that appends `review.stale`.
- If `replan record --supersedes <target>` lands after peek but before `review record` acquires the mutation lock, the closure checks `require_plan_locked` before reclassification and returns `PLAN_INVALID`.
- No `review.stale` or locked-category stale audit event is appended.

Repro status: reproduced with a temp mission and large findings file to hold the pre-lock window.

Disposition before finding review: likely valid P2; may need repair alongside review record locked classification.

### F15: Top-level mission truth files are trusted through symlinks outside `PLANS/<mission>`

Raw severity: P2

Reporter: R10-02

Affected files:
- `crates/codex1/src/state/mod.rs`
- `crates/codex1/src/core/mission.rs`
- `crates/codex1/src/cli/plan/check.rs`
- `crates/codex1/src/cli/task/lifecycle.rs`
- `crates/codex1/src/cli/outcome/validate.rs`

Evidence:
- `resolve_mission` accepts a mission when `STATE.json.is_file()`, which follows symlinks.
- `state::load` reads `STATE.json` without rejecting symlinks.
- Plan and outcome readers use the same `is_file` / `read_to_string` pattern.
- Repro 1: replace `PLANS/demo/STATE.json` with symlink to external JSON where `mission_id:"outside"`, `revision:42`, loop active. `codex1 status --mission demo` returns mission id `outside`, revision `42`, and external loop state.
- Repro 2: replace `PLANS/demo/PLAN.yaml` with symlink to external valid plan. `plan check --mission demo` succeeds and locks it.

Repro status: reproduced in `/tmp`.

Disposition before finding review: likely valid P2; related but not duplicate of round-9 sidecar write symlink issue.

### F16: Review commands can complete a review before the review task's own DAG dependencies are ready

Raw severity: P2

Reporter: R10-07

Affected files:
- `crates/codex1/src/cli/review/plan_read.rs`
- `crates/codex1/src/cli/review/start.rs`
- `crates/codex1/src/cli/review/record.rs`

Evidence:
- Plan validation allows a review task to have extra `depends_on` entries beyond `review_target.tasks`; it only requires targets be included.
- Review command `PlanTask` does not parse `depends_on`.
- `review start` validates only `review_target.tasks`, not all review task dependencies.
- Repro: review `T5` has `depends_on:[T2,T4]`, `review_target.tasks:[T2]`; `T2` is awaiting review, `T4` pending. `task next` correctly returns `run_task T4`, but `review start T5` and `review record T5 --clean` succeed. `T5` becomes complete while `T4` remains pending, and after finishing `T4`, status reports ready for mission-close without requiring another review.

Repro status: reproduced in `/tmp/codex1-review-deps.vTxQyV`.

Disposition before finding review: likely valid P2.

### F17: Harmless state mutations while a review is open make valid findings audit-only

Raw severity: P2

Reporter: R10-07

Affected files:
- `crates/codex1/src/cli/review/start.rs`
- `crates/codex1/src/cli/review/classify.rs`
- `crates/codex1/src/cli/review/record.rs`

Evidence:
- `review start` stores boundary revision as the post-start revision.
- `classify` returns `late_same_boundary` whenever current state revision is greater than boundary revision, regardless of whether the review remains pending and targets are unchanged.
- Repro: start review, then run unrelated `loop activate --mode review_loop`, then record dirty findings. `review record` returns `category:"late_same_boundary"`, no findings file, no dirty counter, and state review remains pending.

Repro status: reproduced in `/tmp/codex1-review-boundary.KS9qoZ`.

Disposition before finding review: likely valid P2, but reviewers should examine the intended stale/late contract carefully because prior accepted findings intentionally made late records audit-only.

