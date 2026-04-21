# Round 14 Repair Plan

Baseline: `d88ecda3f1098a4cc8eb4bff2a0e9368da762d49`

Repair scope: the accepted round-14 P1/P2 findings from `meta-review.md`, plus tests/docs directly required by those repairs. Merge-target findings from already-accepted earlier families should be fixed where the same codepaths are hot, but they are not counted as standalone round-14 items.

## Accepted Findings To Repair

P1:

- F12: locked-plan execution/readiness surfaces still ignore `state.plan.hash`, so post-lock `PLAN.yaml` edits can change live work without replan/relock.

P2:

- F02: `task next` docs still advertise `REPLAN_REQUIRED` as an error even though the runtime returns a success envelope with `next.kind = "replan"`.
- F03: task lifecycle docs still publish PascalCase statuses that the runtime no longer emits.
- F04: bare mission discovery still counts symlinked non-missions as real candidates.
- F06: `outcome ratify` rejects valid indented YAML frontmatter even though `outcome check` accepts it as ratifiable.
- F07: forbidden workflow-policy fields can still be ratified into `OUTCOME.md`.
- F16: Ralph hook still fails open on explicit selector errors (`CODEX1_MISSION`, `CODEX1_REPO_ROOT`).
- F17: `$autopilot`’s published flow still skips the required `close check` gate before `close complete`.
- F18: `$review-loop`’s mission-close workflow still requires undefined `CLOSEOUT-preview` / proof-index artifacts.

## Merge-Target Repairs To Fold In Opportunistically

These are already-open bug families that touch the same files and should be repaired while the relevant codepaths are hot:

- round-11 F10 / round-13 F10 family: replan-triggered missions still leak executable work through `status` and `task start`.
- round-11 F14 / round-13 F15 family: superseded dirty review truth survives replan/relock.
- round-10 F14 / round-13 F13 family: late review results during unlocked replan still miss stale-audit classification.
- round-11 F09 / round-13 F09 family: dirty reviews still over-advertise rerunning the review before repair.
- round-10 F16 / round-13 F16 family: review dependency readiness still over-admits non-target `AwaitingReview` dependencies.
- round-11 F19 / round-13 F12 family: planned-review restart still lacks a hard boundary fence.
- round-11 F20 / round-13 F17 family: mission-close review still lacks round identity.
- round-12 F14 / round-13 F08 family: close artifacts still publish before commit under post-precommit failure shapes.
- round-11 F18 / round-12 F18 family: `CLOSEOUT.md` mission-close history truth still has adjacent read-side poisoning risk.
- round-8 F11 docs family: ambiguous-Ralph docs/runtime drift remains stale.
- round-6 P2-8 docs family: stale “Phase B / NOT_IMPLEMENTED” wording remains in top-level docs.

## Repair Groups

### 1. Locked Plan Authority

Findings:

- F12
- merged replan-gate family (round-11 F10)

Intended behavior:

- Once the plan is locked, all execution/readiness surfaces must honor the locked plan snapshot, not any later on-disk `PLAN.yaml` drift.
- `task next`, `task start`, `review start`, `review record`, `status`, `plan waves`, and `plan graph` must fail closed when the current `PLAN.yaml` hash no longer matches `state.plan.hash`.
- While touching those paths, ensure replan-triggered missions do not still advertise or start stale-plan work.

Files to edit:

- `crates/codex1/src/state/mod.rs`
- `crates/codex1/src/cli/task/start.rs`
- `crates/codex1/src/cli/task/next.rs`
- `crates/codex1/src/cli/task/lifecycle.rs`
- `crates/codex1/src/cli/review/start.rs`
- `crates/codex1/src/cli/review/record.rs`
- `crates/codex1/src/cli/review/plan_read.rs`
- `crates/codex1/src/cli/status/mod.rs`
- `crates/codex1/src/cli/status/project.rs`
- `crates/codex1/src/cli/plan/waves.rs`
- `crates/codex1/src/cli/plan/graph.rs`

Tests:

- Post-lock `PLAN.yaml` edit without replan makes `task next`, `task start`, and `status` refuse the drifted plan.
- Same drift blocks review surfaces and read-only DAG surfaces consistently.
- Replan-triggered locked missions do not surface `ready_tasks` and cannot `task start`.

Risks:

- Preserve the existing same-hash idempotent relock path.
- Keep explicit/handled error contracts stable; prefer existing plan-invalid / mission-invalid vocabulary over inventing a new public error code unless forced.

### 2. Outcome Contract Tightening

Findings:

- F06
- F07

Intended behavior:

- Any `OUTCOME.md` that passes `outcome check` must be ratifiable unless blocked by a legitimate state precondition.
- Valid YAML frontmatter indentation must not be a hidden ratify-only failure mode.
- Forbidden workflow-policy fields (`approval_boundaries`, `autonomy`) must not survive ratification into mission destination truth.

Files to edit:

- `crates/codex1/src/cli/outcome/validate.rs`
- `crates/codex1/src/cli/outcome/check.rs`
- `crates/codex1/src/cli/outcome/ratify.rs`
- `.codex/skills/clarify/SKILL.md` only if the wording needs alignment after the runtime fix

Tests:

- Indented but otherwise valid OUTCOME frontmatter passes both check and ratify.
- `approval_boundaries` / `autonomy` cause `outcome check` / `outcome ratify` to fail with actionable feedback.

Risks:

- Preserve the existing frontmatter/body formatting guarantees and non-destructive ratify rewrite.
- Keep the forbid-list narrow and explicitly tied to the handoff, not a generic unknown-key ban.

### 3. Mission Resolution And Ralph Error Handling

Findings:

- F04
- F16

Intended behavior:

- Bare mission discovery must count only valid non-symlink missions as candidates.
- Explicit selector errors in Ralph (`CODEX1_MISSION`, `CODEX1_REPO_ROOT`) must not silently allow Stop.
- While touching this area, consider aligning the ambiguous-mission docs/runtime wording if the code naturally allows it.

Files to edit:

- `crates/codex1/src/core/mission.rs`
- `crates/codex1/src/core/paths.rs` if a shared helper is useful
- `scripts/ralph-stop-hook.sh`
- `scripts/README-hook.md`
- `docs/install-verification.md`
- `README.md` if the Ralph contract wording must be tightened there too

Tests:

- Bare `status` ignores symlinked fake missions during single-mission discovery.
- Hook blocks or fails closed on explicit bad `CODEX1_MISSION`.
- Hook blocks or fails closed on explicit bad `CODEX1_REPO_ROOT`.

Risks:

- Preserve the benign allow-stop fallback for truly missing binary / malformed JSON / empty output.
- Avoid changing the already-tested ambiguous bare-mission behavior unless the fix requires docs-only alignment.

### 4. Public Docs And Skill Flow Alignment

Findings:

- F02
- F03
- F17
- F18

Intended behavior:

- Public CLI docs must reflect the actual `task next` and task-status wire shapes.
- `$autopilot` must describe a terminal-close flow that matches the handoff and its own guardrails.
- `$review-loop` mission-close guidance must only require artifacts the public product actually provides, or it must explicitly describe how to assemble the packet from existing surfaces.
- While touching top-level docs, fold in the stale “Phase B / NOT_IMPLEMENTED” drift if the same files are hot.

Files to edit:

- `docs/cli-reference.md`
- `README.md`
- `docs/install-verification.md`
- `.codex/skills/autopilot/SKILL.md`
- `.codex/skills/autopilot/references/autopilot-state-machine.md`
- `.codex/skills/review-loop/SKILL.md`
- `.codex/skills/review-loop/references/reviewer-profiles.md`

Tests:

- No new docs-only harness is required unless a tiny grep/assertion already exists naturally.
- Keep runtime tests green after any wording-driven flow adjustments.

Risks:

- Keep docs aligned to actual runtime/intent, not to unimplemented aspirational surfaces.
- Avoid broad skill rewrites; repair only the contradictory terminal-close and mission-close-review guidance.

## Verification

Run after implementation:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
make verify-contract
```

Because this pass touches core execution/readiness behavior, outcome validation, Ralph, and public docs/skills, `make verify-contract` is required.
