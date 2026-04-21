# Round 6 Repair Plan

Date: 2026-04-21

This plan follows the `gpt-5.4` meta-review in `docs/audits/round-6/meta-review.md`.

## Accepted Repair Set

### P1

1. Mission ids can escape the `PLANS/` tree.
2. `PLAN.yaml` spec paths can read outside the mission.
3. `close complete` can expose terminal state before `CLOSEOUT.md` is durable/recoverable.
4. `close record-review --findings-file` can advance mission-close dirty counters before findings are durable.
5. `close complete` and `close record-review` can hide stale `--expect-revision` conflicts behind readiness errors.

### P2

1. `status` reports unsafe ready waves as parallel-safe and hides blockers.
2. `task next` computes wave ids in plan order and omits unsafe-wave blockers.
3. No-mission `status` emits `foundation_only: false`.
4. Terminal planned-review records are dropped instead of audited.
5. Mission-close clean review does not reset `__mission_close__` dirty counter.
6. README / CLI reference contain stale `NOT_IMPLEMENTED` phase status.
7. `docs/cli-contract-schemas.md` mutation protocol describes the old state-before-events order.

### Cleanup

1. `.codex/skills/execute/SKILL.md` has stale prose saying `task next` cannot surface `replan`.

## Dropped From Repair Set

- Autopilot blocked-state escalation: meta-review found the skill dispatches by `next_action.kind`, with explicit `repair` and `replan` rows. The finding was a misread.
- Handoff missing `loop activate` / `close record-review`: meta-review found both are already documented.

## Implementation Order

1. Path containment.
   - Add mission id validation in mission resolution before `MissionPaths::new`.
   - Add a path containment helper for mission-relative artifacts.
   - Reject absolute or escaping `tasks[].spec` paths in `plan check`.
   - Use the helper in task/review packet readers before reading specs.
   - Tests: invalid mission id on `init`; escaping spec path rejected by `plan check`; packet commands fail closed if a previously-locked bad plan is present.

2. Close consistency and dirty counters.
   - Enforce `--expect-revision` immediately after state load in `close complete` and `close record-review`.
   - Write/stage dirty mission-close findings before mutating state.
   - Make `close complete` write `CLOSEOUT.md` before terminal mutation when non-terminal, and regenerate missing `CLOSEOUT.md` for already-terminal missions.
   - Reset `__mission_close__` dirty counter on clean.
   - Tests: stale revision wins over readiness; clean resets counter; already-terminal missing closeout is repaired; dirty review refuses missing/unwritable findings before state mutation.

3. Shared wave safety.
   - Extend status/task wave parsing to carry `exclusive_resources` and `unknown_side_effects`.
   - Factor topological depth and parallel-safety computation into a shared helper or align both public surfaces with the existing `plan waves` behavior.
   - Tests: status and task next report `parallel_safe: false` plus blockers; out-of-order DAG gets the same wave id as `plan waves`.

4. Audit/status drift.
   - Change no-mission status fallback to `foundation_only: true`.
   - Audit terminal planned-review records through an event before returning `TERMINAL_ALREADY_COMPLETE`.
   - Tests: no-mission fallback flag; contaminated terminal review appends an event without mutating current truth.

5. Docs and skill cleanup.
   - Remove stale Phase-B `NOT_IMPLEMENTED` language from README / CLI reference.
   - Update mutation protocol docs to EVENTS-before-STATE.
   - Update `$execute` prose about `task next` and `replan`.

## Verification

Run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

If code paths affecting installed behavior change substantially, also run:

```bash
make verify-contract
```
