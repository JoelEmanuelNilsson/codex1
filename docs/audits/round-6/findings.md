# Round 6 Heavy Review Findings

Date: 2026-04-21

This review deployed 10 read-only reviewer lanes using `gpt-5.4-mini` across CLI contracts, plan/DAG, task lifecycle, review/replan, close/status/Ralph, state persistence, skills, install/E2E, test adequacy, and security/path handling. Every reviewer was instructed to read `README.md` and all markdown files under `docs/` before judging code, to use prior audit decisions as intended-state context, and to report only verified P0/P1/P2 findings.

Baseline verification before review:

- `cargo fmt --check` passed.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo test` passed.

Reviewer lanes that returned `NONE`:

- Test adequacy.
- Install / E2E / UX.

## Summary

- P0: 0
- P1: 7
- P2: 8

## P1 Findings

### P1-1: Terminal close is not recoverable if `CLOSEOUT.md` fails after the state commit

Evidence:

- [crates/codex1/src/cli/close/complete.rs](/Users/joel/codex1/crates/codex1/src/cli/close/complete.rs:64) sets `state.close.terminal_at` through `state::mutate`.
- [crates/codex1/src/cli/close/complete.rs](/Users/joel/codex1/crates/codex1/src/cli/close/complete.rs:80) writes `CLOSEOUT.md` after the state commit.
- [docs/mission-anatomy.md](/Users/joel/codex1/docs/mission-anatomy.md:76) and [README.md](/Users/joel/codex1/README.md:65) describe `CLOSEOUT.md` as part of terminal mission truth.

Impact:

If the closeout write fails or the process dies between the state mutation and artifact write, the mission becomes terminal forever without its terminal artifact. The next `close complete` sees `terminal_at` and returns `TERMINAL_ALREADY_COMPLETE`, so the missing artifact is not repaired.

Suggested fix:

Make `close complete` recoverable: check `--expect-revision` before readiness gates, render/write `CLOSEOUT.md` before exposing terminal state, and allow an already-terminal mission with missing closeout to regenerate the artifact instead of returning immediately.

### P1-2: Mission-close dirty review can double-count after a findings-file write failure

Evidence:

- [crates/codex1/src/cli/close/record_review.rs](/Users/joel/codex1/crates/codex1/src/cli/close/record_review.rs:177) mutates state before writing the mission-close findings artifact.
- [crates/codex1/src/cli/close/record_review.rs](/Users/joel/codex1/crates/codex1/src/cli/close/record_review.rs:210) writes `reviews/mission-close-<revision>.md` after the dirty counter has already been incremented.
- [docs/cli-contract-schemas.md](/Users/joel/codex1/docs/cli-contract-schemas.md:320) pins mission-close review recording as a state/artifact transition.

Impact:

If the findings write fails after the state bump, retrying the same dirty review increments the mission-close dirty counter again and can trigger the six-dirty replan threshold early.

Suggested fix:

Stage or copy the findings artifact before the state mutation, preferably to the deterministic next-revision path after validating `--expect-revision`, so state does not advance without the artifact.

### P1-3: Mission ids can escape the `PLANS/` tree

Evidence:

- [crates/codex1/src/core/paths.rs](/Users/joel/codex1/crates/codex1/src/core/paths.rs:29) joins the raw `mission_id` under `PLANS/`.
- Reproduced with `codex1 --json init --repo-root <tmp> --mission ../escape`, which created `<tmp>/escape/STATE.json` outside `<tmp>/PLANS/`.
- [docs/mission-anatomy.md](/Users/joel/codex1/docs/mission-anatomy.md:1) says a mission lives entirely under `PLANS/<mission-id>/`.

Impact:

A mission id containing path separators or `..` can write mission files outside the intended mission root.

Suggested fix:

Validate mission ids before constructing `MissionPaths`. Reject empty strings, path separators, `.` / `..`, and other path-component forms. Add tests for `init --mission ../escape` and resolution with invalid ids.

### P1-4: `PLAN.yaml` spec paths can read outside the mission

Evidence:

- [crates/codex1/src/cli/plan/check.rs](/Users/joel/codex1/crates/codex1/src/cli/plan/check.rs:540) resolves `tasks[].spec` with `paths.mission_dir.join(spec_rel)` and only checks `is_file()`.
- The security reviewer reproduced `codex1 review packet T2` returning a `spec_excerpt` from a file outside the mission tree via a `../../secret.txt` spec path.
- [docs/mission-anatomy.md](/Users/joel/codex1/docs/mission-anatomy.md:64) defines task specs as `specs/T<id>/SPEC.md` under the mission.

Impact:

A locked plan can cause task/review packet commands to read arbitrary local files visible to the current user and expose their contents in JSON output.

Suggested fix:

Reject absolute spec paths and any relative spec path that escapes `mission_dir` after normalization/canonicalization. Apply the guard in `plan check`, and preferably in packet readers as defense in depth.

### P1-5: Autopilot escalates recoverable blocked states

Evidence:

- [.codex/skills/autopilot/SKILL.md](/Users/joel/codex1/.codex/skills/autopilot/SKILL.md:52) routes every `blocked` verdict to user escalation.
- [crates/codex1/src/cli/status/project.rs](/Users/joel/codex1/crates/codex1/src/cli/status/project.rs:151) can emit `next_action.kind: replan` for a blocked state.
- [.codex/skills/autopilot/references/autopilot-state-machine.md](/Users/joel/codex1/.codex/skills/autopilot/references/autopilot-state-machine.md:10) already routes repair/replan states into recovery flow.

Impact:

`$autopilot` can stop on dirty-review or replan-triggered missions instead of following the normal recovery loop.

Suggested fix:

Update the dispatch table so blocked states with `next_action.kind` of `repair` or `replan` route to `$execute` or `$plan replan`; reserve user escalation for genuinely blocked states without a recovery next action.

### P1-6: Close commands can hide stale-writer conflicts behind readiness errors

Evidence:

- [crates/codex1/src/cli/close/complete.rs](/Users/joel/codex1/crates/codex1/src/cli/close/complete.rs:31) checks terminal/readiness before `state::check_expected_revision`.
- [crates/codex1/src/cli/close/record_review.rs](/Users/joel/codex1/crates/codex1/src/cli/close/record_review.rs:57) checks mission-close readiness before `state::check_expected_revision`.
- Reviewer reproduced `close complete --expect-revision 999` and `close record-review --clean --expect-revision 999` returning `CLOSE_NOT_READY` rather than `REVISION_CONFLICT` on an unready mission.
- [docs/cli-contract-schemas.md](/Users/joel/codex1/docs/cli-contract-schemas.md:136) promises strict equality stale-writer protection for mutating commands.

Impact:

Callers that rely on `REVISION_CONFLICT` to detect stale writes may receive a readiness error instead and make decisions against an old state snapshot.

Suggested fix:

Call `state::check_expected_revision(ctx.expect_revision, &current)?` immediately after loading state in both commands, before terminal/readiness gating.

### P1-7: Handoff omits live loop/review verbs

Evidence:

- [docs/codex1-rebuild-handoff/02-cli-contract.md](/Users/joel/codex1/docs/codex1-rebuild-handoff/02-cli-contract.md:59) lists the command surface.
- [crates/codex1/src/cli/loop_/mod.rs](/Users/joel/codex1/crates/codex1/src/cli/loop_/mod.rs:24) includes `loop activate`.
- [crates/codex1/src/cli/close/mod.rs](/Users/joel/codex1/crates/codex1/src/cli/close/mod.rs:43) includes `close record-review`.
- [.codex/skills/review-loop/SKILL.md](/Users/joel/codex1/.codex/skills/review-loop/SKILL.md:48) depends on `close record-review`.

Impact:

A future implementer following the handoff literally may miss the canonical loop activation path and the only mission-close review write path.

Suggested fix:

Add `codex1 loop activate` and `codex1 close record-review` to the documented minimal surface in the handoff file.

## P2 Findings

### P2-1: `status` reports unsafe waves as parallel-safe and hides blockers

Evidence:

- [crates/codex1/src/cli/status/project.rs](/Users/joel/codex1/crates/codex1/src/cli/status/project.rs:55) sets `parallel_safe` to true whenever a wave exists.
- [crates/codex1/src/cli/status/project.rs](/Users/joel/codex1/crates/codex1/src/cli/status/project.rs:68) always emits an empty `parallel_blockers` list.
- [crates/codex1/src/cli/plan/waves.rs](/Users/joel/codex1/crates/codex1/src/cli/plan/waves.rs:234) already computes blockers from `exclusive_resources` and `unknown_side_effects`.
- Reviewer reproduced a valid plan where `plan waves` emitted `parallel_safe: false` and `blockers: ["exclusive_resource:shared-db"]`, while `status` emitted `parallel_safe: true` and no blockers.

Impact:

Ralph-facing status can tell `$execute` that a wave is safe to parallelize when the plan explicitly says it is not.

Suggested fix:

Teach status plan parsing to include `exclusive_resources` and `unknown_side_effects`, compute blocker metadata for the current ready wave, and pass it into both top-level `parallel_safe`/`parallel_blockers` and `next_action.parallel_safe`.

### P2-2: `task next` can mislabel the ready wave when `PLAN.yaml` is not topologically ordered

Evidence:

- [crates/codex1/src/cli/task/next.rs](/Users/joel/codex1/crates/codex1/src/cli/task/next.rs:111) computes wave id with a single pass over plan order.
- Two reviewers reproduced valid out-of-order plans where `plan waves` reported `current_ready_wave: W3`, while `task next` returned `wave_id: W2`.
- [crates/codex1/src/cli/status/next_action.rs](/Users/joel/codex1/crates/codex1/src/cli/status/next_action.rs:183) already uses a fixed-point topological-depth computation.

Impact:

The ready tasks are correct, but the wave label differs between `task next`, `status`, and `plan waves`, breaking the stable command contract.

Suggested fix:

Replace `task next`'s order-dependent wave id logic with topological-depth logic or shared wave derivation.

### P2-3: No-mission `status` flips `foundation_only` the wrong way

Evidence:

- [crates/codex1/src/cli/status/mod.rs](/Users/joel/codex1/crates/codex1/src/cli/status/mod.rs:38) emits the no-mission graceful fallback.
- [crates/codex1/src/cli/status/mod.rs](/Users/joel/codex1/crates/codex1/src/cli/status/mod.rs:45) sets `"foundation_only": false`.
- Reviewer verified `/tmp` no-mission status returns `foundation_only: false`, while docs/audits prior expectations describe the fallback as foundation-only.

Impact:

Small contract drift in the Ralph-safe fallback envelope.

Suggested fix:

Set `foundation_only: true` for the no-mission fallback, or remove/update the field in docs and tests. The narrower code fix is preferred.

### P2-4: Terminal planned-review records are dropped instead of audited

Evidence:

- [crates/codex1/src/cli/review/record.rs](/Users/joel/codex1/crates/codex1/src/cli/review/record.rs:88) returns `TERMINAL_ALREADY_COMPLETE` for `contaminated_after_terminal` before appending any event.
- [docs/mission-anatomy.md](/Users/joel/codex1/docs/mission-anatomy.md:117) says all four late-output categories are appended to `EVENTS.jsonl`.
- [docs/cli-contract-schemas.md](/Users/joel/codex1/docs/cli-contract-schemas.md:164) says contaminated records are appended for audit but do not change mission truth.

Impact:

A late reviewer response after terminal completion disappears from the audit trail, contrary to the late-output contract.

Suggested fix:

Append a no-op audit event for terminal-contaminated review records before returning the terminal error. Preserve mission truth.

### P2-5: Execute skill hides the real replan surface

Evidence:

- [.codex/skills/execute/SKILL.md](/Users/joel/codex1/.codex/skills/execute/SKILL.md:24) says `task next` cannot surface `repair` or `replan`.
- [crates/codex1/src/cli/task/next.rs](/Users/joel/codex1/crates/codex1/src/cli/task/next.rs:20) does emit `kind: replan` when `state.replan.triggered` is set.

Impact:

A future `$execute` invocation can misread the command boundary and miss the mandatory planning handoff.

Suggested fix:

Update the skill prose to say `status` is authoritative and `task next` can also surface the current `replan` handoff, while repair is status-only.

### P2-6: `close record-review --clean` does not reset the mission-close dirty counter

Evidence:

- [crates/codex1/src/cli/close/record_review.rs](/Users/joel/codex1/crates/codex1/src/cli/close/record_review.rs:108) sets `review_state = Passed` on clean.
- Existing [crates/codex1/tests/close.rs](/Users/joel/codex1/crates/codex1/tests/close.rs:577) comments assert the dirty counter stays where the previous dirty record left it.
- [docs/cli-contract-schemas.md](/Users/joel/codex1/docs/cli-contract-schemas.md:166) says accepted-current clean records reset the dirty counter to 0.

Impact:

This is mission-close rather than planned-review state, but the same dirty-counter semantics are intended. Keeping stale mission-close counts after a clean pass makes replan telemetry misleading and could affect later reopen/retry behavior.

Suggested fix:

Reset `replan.consecutive_dirty_by_target["__mission_close__"]` to 0 when a mission-close clean review is recorded, and update the existing test expectation.

### P2-7: `task next` omits parallel blockers on unsafe ready waves

Evidence:

- [crates/codex1/src/cli/task/next.rs](/Users/joel/codex1/crates/codex1/src/cli/task/next.rs:93) emits `"parallel_safe": true` for every multi-task ready wave.
- [docs/cli-contract-schemas.md](/Users/joel/codex1/docs/cli-contract-schemas.md:234) allows `task next` to return `parallel_safe`.
- [crates/codex1/src/cli/plan/waves.rs](/Users/joel/codex1/crates/codex1/src/cli/plan/waves.rs:234) has the needed blocker analysis.

Impact:

Even if status is fixed, a skill or caller using `task next` directly can still parallelize tasks that share exclusive resources or have unknown side effects.

Suggested fix:

Compute the current ready wave's safety in `task next` using the same rules as `plan waves`, and include blockers when unsafe.

### P2-8: Documentation and README still contain stale Phase-B `NOT_IMPLEMENTED` status claims

Evidence:

- [README.md](/Users/joel/codex1/README.md:5) says Phase B commands currently return `NOT_IMPLEMENTED`.
- [docs/cli-reference.md](/Users/joel/codex1/docs/cli-reference.md:10) says commands return `NOT_IMPLEMENTED` today if Phase B has not merged, and many per-command sections still say "Currently returns `NOT_IMPLEMENTED`".
- Baseline tests and reviewers verified these commands are implemented.

Impact:

The user-facing docs undersell the implemented product and can mislead new agents or humans during installation and review.

Suggested fix:

Refresh README and CLI reference phase-status language to describe the current implemented state.

## Notes

- The path findings were independently validated by the main thread before aggregation.
- The wave-id mismatch was independently reported by two reviewer lanes.
- Test adequacy and install/E2E lanes did not report additional P0/P1/P2 findings.
