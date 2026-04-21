# Round 15 Repair Plan

Baseline: `1ac8ca37239194fb70f296c2c61b50bb22de9ce2`

Repair scope: accepted round-15 P1/P2 findings from `meta-review.md`, plus tests and docs directly required by those repairs. Merge-target historical families may be fixed where they share the same codepath, but they are not counted as standalone round-15 items.

## Accepted Findings To Repair

P1:

- F01: clap-validated argv errors still bypass the JSON envelope / exit-code contract.
- F10: `task next` advertises rerunning a dirty review instead of the required repair task.
- F12: restarting a planned review boundary still accepts stale findings from the previous review round.
- F14: dirty planned-review artifacts can publish before the state mutation commits.
- F15: `close check` / `close complete` bypass locked-plan drift while `status` reports `invalid_state`.
- F17: terminal missions are still mutable through public task and loop commands.
- F23: `outcome ratify` can leave `STATE.json` ratified while `OUTCOME.md` is still draft if the file write fails.

P2:

- F03: absolute proof paths that point inside the mission bypass mission-local symlink containment.
- F05: outcome validation still misses longer placeholder/vague variants like `TODO: fill this in` and `Workflow is reliable.`
- F06: pure-YAML `OUTCOME.md` files are still rejected even though the primary handoff allows them.
- F09: replacement plans can reuse untouched historical task IDs from the prior locked DAG snapshot.
- F11: `plan waves` / `plan graph` advertise executable work while the mission is globally blocked.
- F13: late review results during the unlocked replan window are still rejected before stale audit classification.
- F24: event append-before-state ordering can duplicate `EVENTS.jsonl` sequence numbers after handled post-append failure and retry.

## Repair Groups

### 1. CLI And Mutation Atomicity

Findings:

- F01
- F23
- F24

Intended behavior:

- Clap argv validation errors must return canonical JSON error envelopes and handled-error exit code `1`.
- `outcome ratify` must not commit ratified state unless `OUTCOME.md` can also be written.
- Event/state commits must not create duplicate event sequence numbers on retry after a handled post-append failure.

Files to edit:

- `crates/codex1/src/cli/mod.rs`
- `crates/codex1/src/cli/outcome/ratify.rs`
- `crates/codex1/src/state/mod.rs`
- `crates/codex1/src/state/events.rs`
- `crates/codex1/tests/outcome.rs`
- focused CLI contract tests if an existing test module fits naturally

Tests:

- Missing required command args produce JSON `PARSE_ERROR` and exit `1`, not raw clap exit `2`.
- Ratify preflights `OUTCOME.md` writability before mutating state.
- Event append rejects or recovers from existing trailing duplicate/advanced lines so retry cannot append a duplicate `seq`.

### 2. Outcome And Proof Contract

Findings:

- F03
- F05
- F06

Intended behavior:

- Pure-YAML and fenced-frontmatter `OUTCOME.md` forms both validate and ratify.
- Placeholder/vague criteria detection catches obvious longer variants while avoiding broad semantic policing.
- Absolute proof paths that resolve inside the mission must pass the same symlink/containment checks as relative mission-local paths.

Files to edit:

- `crates/codex1/src/cli/outcome/validate.rs`
- `crates/codex1/src/cli/outcome/ratify.rs`
- `crates/codex1/src/core/paths.rs`
- `crates/codex1/tests/outcome.rs`
- `crates/codex1/tests/task.rs`

Tests:

- Pure-YAML valid outcome passes check and ratify, preserving pure-YAML shape.
- `TODO: fill this in` and `Workflow is reliable.` block check/ratify.
- Mission-local absolute proof symlink escape is rejected.

### 3. Readiness And Terminal Guards

Findings:

- F10
- F11
- F15
- F17

Intended behavior:

- Dirty review findings make repair the next executable action until target work is repaired.
- Graph and waves should not advertise currently executable ready work while replan or dirty-review blockers are active.
- Close check/complete must enforce the locked-plan snapshot hash.
- Terminal missions must reject task and loop mutations, while preserving audit-only late review contamination paths.

Files to edit:

- `crates/codex1/src/state/mod.rs`
- `crates/codex1/src/state/readiness.rs`
- `crates/codex1/src/cli/task/next.rs`
- `crates/codex1/src/cli/task/start.rs`
- `crates/codex1/src/cli/loop_/mod.rs`
- `crates/codex1/src/cli/close/check.rs`
- `crates/codex1/src/cli/close/complete.rs`
- `crates/codex1/src/cli/plan/waves.rs`
- `crates/codex1/src/cli/plan/graph.rs`
- `crates/codex1/tests/task.rs`
- `crates/codex1/tests/loop_.rs`
- `crates/codex1/tests/close.rs`
- `crates/codex1/tests/plan_waves.rs`

Tests:

- Dirty review makes `task next` return repair, not `run_review`.
- Replan/dirty blockers suppress executable readiness in waves/graph.
- Locked-plan drift blocks close check and close complete.
- Terminal missions reject task start and loop activate with `TERMINAL_ALREADY_COMPLETE`.

### 4. Review And Replan Boundaries

Findings:

- F09
- F12
- F13
- F14

Intended behavior:

- Replacement plans cannot reuse historical task IDs from the previous locked DAG snapshot unless the ID is explicitly still live and unchanged under a documented rule.
- Planned review restarts must fence stale old findings from mutating current truth.
- Late review output during unlocked replan must still emit stale audit classification.
- Dirty review findings artifacts must be published only after the corresponding state/event commit is safe.

Files to edit:

- `crates/codex1/src/cli/plan/check.rs`
- `crates/codex1/src/cli/review/classify.rs`
- `crates/codex1/src/cli/review/record.rs`
- `crates/codex1/src/state/schema.rs` only if a durable boundary token is unavoidable
- `crates/codex1/tests/plan_check.rs`
- `crates/codex1/tests/review.rs`

Tests:

- Replan relock rejects reuse of untouched historical IDs from `state.plan.task_ids`.
- Replayed findings after `review start` restart are not `accepted_current`.
- Review record during unlocked replan records stale audit instead of `PLAN_INVALID`.
- Artifact publication is ordered after state/event commit or can be recovered safely without orphaning visible review truth.

## Verification

Run after implementation:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
make verify-contract
```
