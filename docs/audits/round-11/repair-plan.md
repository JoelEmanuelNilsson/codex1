# Round 11 Repair Plan

Baseline: `6c33536 round-10 repairs: lock lifecycle truth`

Repair scope: the accepted round-11 P1/P2 findings from `meta-review.md`, plus tests/docs directly required by those repairs. Merge-target findings from earlier accepted families may be fixed opportunistically when touching the same codepaths, but are not counted as standalone round-11 items.

## Accepted Findings To Repair

P1:

- F07: `plan check` relocks replans that reuse historical task IDs.
- F09: dirty planned reviews are still advertised as ready review work before repair is complete.
- F10: a triggered replan does not actually block stale-plan execution.
- F14: superseded dirty planned reviews remain current blockers after replan.
- F19: restarting a planned review does not fence off late results from the previous boundary.
- F20: mission-close review has no round identity, so a stale clean can still pass the terminal gate.
- F23: `$autopilot` cannot actually finish autonomously because it still asks for extra close confirmation.

P2:

- F01: `task next` keeps advertising `mission_close_review` after close is ready or terminal.
- F02: review docs are stale for emitted `review start` / `review status` envelopes.
- F03: `plan graph` / `plan waves` still trust a symlinked `PLAN.yaml`.
- F04: packet and closeout paths still trust a symlinked `OUTCOME.md`.
- F06: OUTCOME `mission_id` is not checked against the active mission.
- F08: `plan check` still accepts stored `waves:` truth.
- F13: `task next` tells a fresh mission to plan instead of clarify.
- F16: `close record-review --findings-file` returns the wrong public error contract.
- F17: post-terminal mission-close review results are dropped instead of audited.
- F18: `CLOSEOUT.md` can falsely claim the mission-close review was clean on the first round.
- F21: `status` swallows explicit `--repo-root` discovery failures into the no-mission fallback.
- F22: public `status` docs still advertise the pre-repair ambiguity behavior.
- F24: `verify-installed` can still pass even when `codex1` is unusable from `/tmp` via PATH.
- F25: `doctor` reports a non-executable `codex1` file as “on PATH”.
- F26: `review packet` docs promise proof paths the CLI never emits.

## Merge-Target Repairs To Fold In Opportunistically

These are already-open bug families that touch the same files and should be fixed while the code is hot:

- round-10 F11 family: post-repair stale dirty-review routing (`status` verdict, `task start` reopening repaired targets).
- round-10 F14 family: stale review audit after replan unlock.
- round-10 F16 family: review dependency readiness for non-target dependencies.
- round-9 F03 family: empty required OUTCOME fields still ratify.

## Repair Groups

### 1. Review Boundary And Mission-Close Identity

Findings:
- F14
- F19
- F20
- merged F15
- merged F27

Intended behavior:
- Replan or review restart must retire/fence old review truth.
- Only the currently active review boundary may mutate current truth.
- Mission-close review needs the same kind of boundary identity and stale/late handling as planned reviews.
- Dependency readiness for reviews must require non-target deps to be truly review-clean/complete.

Files to edit:
- `crates/codex1/src/cli/review/start.rs`
- `crates/codex1/src/cli/review/record.rs`
- `crates/codex1/src/cli/review/classify.rs`
- `crates/codex1/src/cli/close/record_review.rs`
- `crates/codex1/src/state/schema.rs`
- `crates/codex1/src/state/readiness.rs`
- `crates/codex1/src/cli/status/next_action.rs`

Tests:
- Restarted planned review rejects or audit-classifies stale old results.
- Replan unlock + stale review record emits stale audit instead of `PLAN_INVALID`.
- Superseded dirty reviews no longer block `status`/`close check` after relock.
- Mission-close dirty then stale clean cannot satisfy `close check`.
- Non-target `AwaitingReview` dependency does not make a review ready.

Risks:
- Schema extension for mission-close boundary identity must preserve old state-file compatibility.
- Review classification changes must not re-open the previously fixed “unrelated state mutation makes review late” false positive.

### 2. Replan And Dirty-Review Readiness Surfaces

Findings:
- F09
- F10
- F01
- F13
- merged round-10 F11 family

Intended behavior:
- When repair is required, every public work surface agrees that repair is the only executable next step.
- When replan is triggered, no stale-plan work may be advertised or mutated.
- `task next` must match clarify/close/closed sequencing from the primary readiness contract.

Files to edit:
- `crates/codex1/src/cli/task/next.rs`
- `crates/codex1/src/cli/task/start.rs`
- `crates/codex1/src/cli/task/lifecycle.rs`
- `crates/codex1/src/cli/status/project.rs`
- `crates/codex1/src/cli/status/next_action.rs`
- `crates/codex1/src/cli/plan/waves.rs`
- `crates/codex1/src/cli/plan/graph.rs`

Tests:
- Dirty review before repair: `status`, `task next`, `plan waves`, `plan graph`, and `review start` all agree repair is next.
- After repair finish: no stale `blocked + run_review`, no reopened repair via `task start`.
- Replan triggered: `status`, `task next`, `plan waves`, and `task start` all fail closed on stale-plan work.
- Fresh init: `task next` reports `clarify`.
- Mission-close passed and terminal: `task next` reports `close` / `closed`, not `mission_close_review`.

Risks:
- We need one shared notion of “repair still needed” and one shared notion of “execution allowed” to avoid creating the next drift across surfaces.

### 3. Plan And Outcome Truth Integrity

Findings:
- F07
- F06
- F08
- merged round-9 F03 family

Intended behavior:
- Replans must not reuse historical task IDs.
- OUTCOME truth must match the active mission and satisfy non-empty required-field rules.
- `PLAN.yaml` must reject forbidden stored-wave truth instead of silently blessing it.

Files to edit:
- `crates/codex1/src/cli/plan/check.rs`
- `crates/codex1/src/cli/outcome/validate.rs`
- `crates/codex1/src/cli/outcome/check.rs`
- `crates/codex1/src/cli/outcome/ratify.rs`
- possibly `crates/codex1/src/cli/plan/parsed.rs`

Tests:
- Reusing a historical task ID after replan fails `plan check`.
- OUTCOME `mission_id` mismatch fails both `outcome check` and `outcome ratify`.
- Empty required OUTCOME lists fail validation consistently.
- `PLAN.yaml` containing `waves:` fails `plan check`.

Risks:
- Preserve the idempotent locked-plan path while rejecting genuine replan reuse.
- Keep outcome validation messages actionable rather than turning one family of validator failures into opaque parse errors.

### 4. Remaining Mission-Truth Containment

Findings:
- F03
- F04
- F21

Intended behavior:
- Every reader of mission-owned truth files must reject symlinked artifacts that escape the mission root.
- Explicit `--repo-root` status calls must surface operator error, not degrade to the bare no-mission fallback.

Files to edit:
- `crates/codex1/src/cli/plan/graph.rs`
- `crates/codex1/src/cli/plan/waves.rs`
- `crates/codex1/src/cli/task/worker_packet.rs`
- `crates/codex1/src/cli/review/packet.rs`
- `crates/codex1/src/cli/close/closeout.rs`
- `crates/codex1/src/cli/status/mod.rs`
- possibly a shared helper in `crates/codex1/src/core/paths.rs`

Tests:
- Symlinked `PLAN.yaml` rejected by `plan graph` and `plan waves`.
- Symlinked `OUTCOME.md` rejected by task packet, review packet, and closeout path.
- `status --repo-root <empty-root>` returns `MISSION_NOT_FOUND` instead of the graceful fallback.

Risks:
- Keep the graceful bare-CWD no-mission fallback for Ralph and casual status use while only tightening explicit selector cases.

### 5. Close Contract And Audit Surfaces

Findings:
- F16
- F17
- F18

Intended behavior:
- Mission-close findings-file validation should use the correct error code family.
- Terminal mission-close late outputs must be audited.
- `CLOSEOUT.md` must tell the truth about mission-close history.

Files to edit:
- `crates/codex1/src/cli/close/record_review.rs`
- `crates/codex1/src/cli/close/closeout.rs`
- possibly `crates/codex1/src/core/error.rs`

Tests:
- Missing mission-close findings file returns the correct contract error.
- Post-terminal mission-close record appends an audit event without mutating truth.
- Dirty-then-clean mission-close history renders accurately in `CLOSEOUT.md`.

Risks:
- If we add a new error variant for close-review findings, docs and tests must be updated together.
- Mission-close history should come from stable truth, not from resettable counters.

### 6. Install, Doctor, And Docs

Findings:
- F02
- F22
- F23
- F24
- F25
- F26

Intended behavior:
- Public docs must match emitted JSON and current mission-resolution behavior.
- `$autopilot` should really be autonomous on the normal terminal-close happy path.
- Installed verification should prove the normal `codex1` invocation path works from `/tmp`.
- `doctor` should not claim a non-executable shadow file is a valid PATH resolution.

Files to edit:
- `docs/cli-reference.md`
- `docs/cli-contract-schemas.md`
- `.codex/skills/autopilot/SKILL.md`
- `.codex/skills/autopilot/references/autopilot-state-machine.md`
- `Makefile`
- `crates/codex1/src/cli/doctor.rs`

Tests:
- `verify-installed` smoke should assert bare `codex1` resolution from `/tmp`.
- `doctor` test for non-executable PATH shadow.
- Existing docs tests are not present; keep changes narrow and consistent with current command shapes.

Risks:
- Avoid rewriting the whole docs set; only correct the specific stale contract examples and flows.

## Verification

Run after implementation:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
make verify-contract
```

Because this pass touches status/task-next behavior, review/close lifecycle logic, install verification, and public docs/skills, `make verify-contract` is required.
