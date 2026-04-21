# Round 8 Heavy Review Findings

Date: 2026-04-21

This round deployed 16 read-only reviewer lanes using `gpt-5.4` with high reasoning. Every reviewer was instructed to read `README.md` and every markdown file under `docs/**/*.md`, use prior audit decisions as intended-state context, avoid previously rejected false positives, mutate no repo mission state, and report only verified P0/P1/P2 findings.

The worktree was clean before the review wave.

## Reviewer Management Table

| Reviewer ID | Surface | Model / reasoning | Result | P0 | P1 | P2 | Finding titles | Evidence quality | Duplicate / unique assessment | Repro status | Main-thread initial disposition | Recommended next step |
| --- | --- | --- | --- | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- |
| R01 | CLI contract and envelopes | gpt-5.4 / high | Findings | 0 | 0 | 2 | `review start` masks stale `--expect-revision`; superseded live-DAG projection disagreement | High | Duplicates R16; live-DAG also overlaps R14/R08/R06 | Reproduced | Candidate P2s | Shard review with regression lane |
| R02 | Mission resolution and path security | gpt-5.4 / high | Findings | 0 | 1 | 0 | CLI-owned mission artifact writes escape through symlinked mission/review dirs | High | Unique write-side security finding | Reproduced | Candidate P1 | Prioritize shard review; likely repair-driving |
| R03 | Outcome and clarify | gpt-5.4 / high | Findings | 0 | 1 | 0 | `outcome check` / `ratify` accept missing required fields | High | Unique runtime validator finding; related to prior doc-only P3 but stronger | Reproduced | Candidate P1 | Shard review validation semantics |
| R04 | Plan validation | gpt-5.4 / high | Findings | 0 | 0 | 1 | Review tasks can omit targets from `depends_on` and be scheduled as normal work | High | Unique DAG validation gap | Reproduced | Candidate P2 | Shard review against plan contract |
| R05 | DAG, waves, graph | gpt-5.4 / high | Findings | 0 | 0 | 1 | `plan waves` advances `current_ready_wave` past in-progress dependency | High | Unique public projection bug | Reproduced | Candidate P2 | Shard review with live-DAG findings |
| R06 | Task lifecycle | gpt-5.4 / high | Findings | 0 | 0 | 3 | Replan strands orphan review task; review packet drops absolute proof; close succeeds after deleted proof | High | Orphan review overlaps R08/F06; missing proof overlaps R15 | Reproduced | Candidate P2s | Shard review, merge duplicates |
| R07 | Review lifecycle | gpt-5.4 / high | Findings | 0 | 2 | 0 | `review start` erases active dirty review; late dirty records become current blockers | High | Unique review-truth findings | Reproduced | Candidate P1s | Shard review for intended late-output semantics |
| R08 | Replan lifecycle | gpt-5.4 / high | Findings | 0 | 1 | 1 | Replan strands orphan review task; replan docs scalar/array mismatch | High | Orphan review duplicates R06; docs mismatch unique | Reproduced | Candidate P1/P2 | Shard review, severity check |
| R09 | Close lifecycle | gpt-5.4 / high | Findings | 0 | 1 | 0 | Stale mission-close dirty record can reopen after clean | High | Unique concurrency/semantic-stale close finding | Reproduced | Candidate P1 | Shard review with state-concurrency finding |
| R10 | State persistence and concurrency | gpt-5.4 / high | Findings | 0 | 1 | 0 | Concurrent mission-close review stale writers overwrite committed findings artifact | High | Unique artifact/state atomicity finding; related to R09 | Reproduced | Candidate P1 | Shard review; likely repair-driving |
| R11 | Status and Ralph | gpt-5.4 / high | Findings | 0 | 0 | 1 | Ralph fails open on ambiguous multi-mission status errors | High | Unique hook/status integration finding | Reproduced | Candidate P2 | Shard review, consider safe hook behavior |
| R12 | Loop commands and orchestration skills | gpt-5.4 / high | Findings | 0 | 1 | 0 | `$execute` rejects valid `$autopilot`/`status` handoffs | High | Unique skill orchestration finding | Reproduced | Candidate P1 | Shard review, likely skill repair |
| R13 | Install and Makefile UX | gpt-5.4 / high | Findings | 0 | 0 | 2 | `verify-installed` does not verify concrete installed binary / `/tmp` doctor; `install-local` breaks on paths with spaces | High | Install smoke extends round-7; whitespace path unique | Reproduced | Candidate P2s | Shard review with Makefile scope |
| R14 | Test adequacy | gpt-5.4 / high | Findings | 0 | 0 | 1 | Superseded-root downstream live-DAG regression coverage missing | High | Test-gap duplicate of R01/R16 live-DAG bug | Not runtime repro; suite gap confirmed | Merge as test requirement if live-DAG accepted | Add to live-DAG test plan |
| R15 | Docs/handoff cross-check | gpt-5.4 / high | Findings | 0 | 0 | 1 | `close check` / `close complete` allow terminal close after proof deletion | High | Duplicate of R06 proof-close finding | Reproduced | Candidate P2 | Shard review once, merge |
| R16 | Current diff / regression | gpt-5.4 / high | Findings | 0 | 0 | 2 | `review start` stale revision ordering; live-DAG superseded projection mismatch | High | Duplicates R01; live-DAG overlaps R14/R08/R06 | Reproduced | Candidate P2s | Merge with R01/R14 |

Raw reviewer counts before dedupe: P0 0, P1 9, P2 18.

## Deduplicated Raw Finding Index

These are raw candidates for finding-review. The severity here is the reporting reviewer severity, not the final accepted severity.

### F01 P1: CLI-owned mission artifact writes can escape `PLANS/<mission>` through symlinked mission/review directories

Reported by R02. Reproduced by replacing `PLANS/demo/reviews` with a symlink to `/tmp/outside`, then running `review record`; the command reported `PLANS/demo/reviews/T2.md` but created `/tmp/outside/T2.md`. A second variant pre-created `PLANS/demo` as a symlink and `init` wrote `STATE.json`, `OUTCOME.md`, `PLAN.yaml`, `EVENTS.jsonl`, and `STATE.json.lock` outside `PLANS`.

Initial disposition: candidate P1. Needs validation of intended write-side symlink policy and repair breadth.

### F02 P1: `outcome check` and `outcome ratify` accept OUTCOME files missing required `definitions` and `resolved_questions`

Reported by R03. The docs and clarify reference list both fields as required, but `validate_outcome` does not check them. A `/tmp` repro omitted both fields; `outcome check` returned `ratifiable: true`, and `outcome ratify` advanced state to `phase: plan`.

Initial disposition: candidate P1. Distinct from prior skill-summary P3 because the CLI validator itself misses required fields.

### F03 P2: `plan check` locks review tasks whose `depends_on` omits their `review_target.tasks`

Reported by R04. A plan with `T2 kind: review`, `review_target.tasks: [T1]`, and `depends_on: []` locked successfully. `status` and `task next` then returned `run_wave` with `["T1","T2"]`, allowing the review task to be treated as ordinary parallel work before its target.

Initial disposition: candidate P2. Needs validation against the plan contract and current review task lifecycle.

### F04 P2: `plan waves` can report a downstream `current_ready_wave` while an upstream dependency is in progress

Reported by R05. With `T1` in progress and `T2/T3` depending on `T1`, `plan waves` reported `current_ready_wave: W2` while `status` and `task next` blocked. The suspected root is `wave_is_current_ready` checking statuses inside the candidate wave without rechecking dependency satisfaction.

Initial disposition: candidate P2. Public projection disagreement with direct scheduling impact.

### F05 P2: `review start` still masks stale `--expect-revision` behind `PLAN.yaml` parsing

Reported by R01 and R16. `review start --expect-revision 999` with malformed `PLAN.yaml` returns `PLAN_INVALID` instead of `REVISION_CONFLICT` because `review/start.rs` loads review tasks from `PLAN.yaml` before loading state and checking revision.

Initial disposition: candidate P2; duplicate reports agree. This appears to be an incomplete round-7 repair.

### F06 P2: Superseded live-DAG projection still disagrees across `plan waves`, `status`, and `task next`

Reported by R01, R14, and R16. With live `T2` depending on superseded `T1`, `plan waves` returns `DAG_MISSING_DEP`, while `status` and `task next` return successful `blocked` projections. R14 identified the exact regression coverage gap for this accepted round-7 scenario.

Initial disposition: candidate P2; likely accepted or merged with F16 depending final live-DAG rule.

### F07 P1: `review start` can erase an active dirty review and bypass repair

Reported by R07. After `review record T2 --findings-file`, `status` reported `blocked` / `repair`. Running `review start T2` succeeded and rewrote the review to `pending`; a subsequent clean review advanced toward mission close without any repair task/proof.

Initial disposition: candidate P1. Needs careful review of whether restarting a review should ever clear dirty truth.

### F08 P1: `late_same_boundary` dirty records are stored as current truth and block/repair the mission

Reported by R07. A late dirty `review record` produced `category: late_same_boundary` and did not bump dirty counters, but stored `reviews.T5.verdict = dirty`; readiness/status then treated it as current blocker and routed to repair. Docs say non-current categories are audit-only and do not mutate current truth.

Initial disposition: candidate P1. Likely repair involves separating audit event from current review record or filtering by category.

### F09 P1: Stale mission-close dirty records can reopen a review after a clean pass

Reported by R09. Concurrent dirty and clean `close record-review` calls can result in clean succeeding first, then dirty succeeding later based on pre-lock readiness, leaving final `close.review_state = open` and blocking `close complete`.

Initial disposition: candidate P1. Needs concurrency validation and repair design with in-lock precondition revalidation.

### F10 P1: Concurrent mission-close review recording lets stale/rejected writers overwrite committed findings artifacts

Reported by R10. Eight concurrent dirty `close record-review --expect-revision 5` calls produced one successful state transition and seven `REVISION_CONFLICT`s, but the final `reviews/mission-close-6.md` contained a rejected writer's content because all callers staged the same pre-lock destination before `state::mutate`.

Initial disposition: candidate P1. Strong artifact/state atomicity issue. Consider planned review record as similar risk.

### F11 P2: Ralph fails open on ambiguous multi-mission status errors

Reported by R11. Bare `status` now correctly errors with `MISSION_NOT_FOUND` / `context.ambiguous:true` for multiple missions, but `scripts/ralph-stop-hook.sh` parses missing `.data.stop.allow` as allow. Repro showed `status --mission a` would block (`stop.allow:false`) while hook invocation without mission exited 0.

Initial disposition: candidate P2. Needs hook/status contract decision for multi-mission repos.

### F12 P1: `$execute` rejects valid `$autopilot` and `status` handoffs

Reported by R12. `status` after plan lock can return `next_action.command: "$execute"` with `loop.active:false`, while `$execute` preconditions require `loop.active:true` and no skill calls `loop activate`. Dirty-review repair yields `verdict: blocked` / `next_action.kind: repair`, which autopilot routes to `$execute`, but `$execute` requires `verdict: continue_required`.

Initial disposition: candidate P1. Skill orchestration may stall normal execution and repair.

### F13 P2: `verify-installed` can pass without verifying the freshly installed binary or documented `/tmp` doctor smoke

Reported by R13. `Makefile verify-installed` invokes plain `codex1` from `PATH`, never asserts it is `$(INSTALL_DIR)/codex1`, and runs `doctor` before the `/tmp` smoke block. A fake `codex1` passed `make verify-installed` while failing `doctor` from `/tmp`.

Initial disposition: candidate P2. Extends accepted round-7 install-smoke repair.

### F14 P2: `install-local` breaks for valid install directories containing spaces

Reported by R13. `Makefile install-local` uses unquoted `$(INSTALL_DIR)` in `mkdir` and `cp`. Repro with `INSTALL_DIR="/tmp/codex1 install.XXXXXX"` failed.

Initial disposition: candidate P2, pending severity review. Fix likely cheap alongside F13.

### F15 P2: `close check` / `close complete` allow terminal close after required task proof is missing

Reported by R06 and R15. A task was finished with `specs/T1/PROOF.md`, the proof file was deleted, then `close record-review --clean`, `close check`, and `close complete` succeeded. `CLOSEOUT.md` cited a missing proof. Handoff says close check requires proof exists.

Initial disposition: candidate P2. Needs decision whether close readiness should revalidate proof artifacts after task finish.

### F16 P1/P2: Replanning a reviewed task can strand an orphan review task with no CLI recovery path

Reported by R06 as P2 and R08 as P1. A replan superseding work `T2` awaiting review can leave planned review `T3` in `plan.task_ids` but absent from `state.tasks`; `task next` hides it, `close check` blocks on it, `task start T3` cannot run, `review record T3` is stale, and `replan record --supersedes T3` rejects unknown ids because it validates only `state.tasks`.

Initial disposition: candidate P1/P2. Likely related to F06 live-DAG/supersession but has a stronger close-blocking lifecycle symptom.

### F17 P2: Review packets drop recorded proof paths when `task finish` used an absolute proof path

Reported by R06. `task finish` accepts and stores absolute proof paths, but `review packet` only checks conventional `specs/<task>/PROOF.md`, so `proofs: []` for a valid finished task whose `proof_path` is absolute.

Initial disposition: candidate P2. Straight contract/packet propagation issue.

### F18 P2: CLI reference documents `replan record.data.supersedes` as scalar while CLI emits an array

Reported by R08. `docs/cli-reference.md` shows `"supersedes":"T4"`, but CLI accepts repeated `--supersedes` and emits `"supersedes":["T4"]` even for one value.

Initial disposition: candidate P2 or doc cleanup. Needs severity review.

## Clean Lanes

No reviewer returned `NONE`; all 16 lanes reported at least one candidate P0/P1/P2. Several are duplicates and may be rejected or downgraded by finding review.

