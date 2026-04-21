# Round 17 Repair Plan

Date: 2026-04-22

Scope: implement only accepted P0/P1/P2 fixes from round 17 plus tightly coupled tests/docs.

## Accepted Repairs

### Close lifecycle and closeout history

- Allow a mission-close review in `open` state to be recorded clean, so a dirty mission-close review can be followed by a clean pass.
- Keep duplicate dirty mission-close records from inflating dirty counters without an intervening repair boundary.
- Compute mission-close dirty artifact filenames from committed mutation revision truth, not from a pre-lock revision estimate.
- Reject or fail closed when closeout history would read a symlinked `reviews/` directory.

Findings covered: F01, F02, F10, F15.

### Artifact durability

- Ensure planned dirty review findings artifacts are durable before state/event truth advances.
- Ensure mission-close dirty findings artifacts are durable before state/event truth advances.
- Add direct regression tests where artifact writes fail and state must not advance.

Findings covered: F07, F11, F15.

### Outcome ratify publication

- Add an event append preflight before `mutate_dynamic_with_precommit` runs artifact publication callbacks, so common event/state commit failures are caught before `OUTCOME.md` is rewritten.
- Add or preserve regression coverage for ratify failure paths that must not leave `OUTCOME.md` ahead of state.

Findings covered: F03.

### Replan, orphan tasks, and status consistency

- Prevent relocked plans from omitting non-terminal, non-superseded tasks in a way that strands mission state outside the locked DAG.
- Allow append-style replacement plans to retain completed prerequisite task IDs while still rejecting unsafe historical ID reuse.
- Ensure status and task-next do not advertise runnable work while orphan blockers are active.
- Allow dirty-review repair readiness to advance consistently once all dirty targets are repaired.

Findings covered: F04, F05, F09, F12.

### Review readiness and stale boundaries

- Centralize review dependency readiness so only actual review target dependencies may be `AwaitingReview`; non-target dependencies must be complete or clean-reviewed.
- Prevent stale outputs from an earlier planned-review boundary from being recorded after `review start` creates a newer pending boundary.
- Strengthen stale-review audit payload coverage for category and target context.

Findings covered: F06, F08, F13.

### Status/Ralph coverage

- Add direct test coverage that active invalid-state status projection fails closed with stop disallowed.

Findings covered: F14.

## Verification Gate

Before committing round 17:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `make verify-contract`

If a repair path proves larger than the accepted severity warrants, prefer the smallest safe fix that closes the verified repro and add a regression test that pins the intended behavior.
