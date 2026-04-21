# Round 3 — handoff-cross-check audit

## Summary

Audited `HEAD` (round-2 fixes committed) against every normative claim and
anti-goal across the seven handoff docs at
`docs/codex1-rebuild-handoff/{README,00-why-and-lessons,01-product-flow,02-cli-contract,03-planning-artifacts,04-roles-models-prompts,05-build-prompt}.md`.

**0 P0, 0 P1, 0 P2, 0 P3.**

Rule-5 reminder: Round-1 REJECTed `02-cli-contract.md:216` list-vs-code
`complete`/`terminal_complete` drift (handoff-internal, handoff frozen).
Round-2 REJECTed F1 `ParsedPlan` `deny_unknown_fields` as non-blocking
P3. Neither is re-surfaced here.

Every declared anti-goal is honored in code, skills, and scripts:

- `find /Users/joel/codex1 -name .ralph` / `-name .ralph*` returns zero
  hits. Mentions live only in anti-goal prose (`crates/codex1/src/lib.rs`,
  `.codex/skills/close/SKILL.md`, every `docs/codex1-rebuild-handoff/*.md`,
  `README.md:3`).
- Caller-identity patterns
  (`is_parent|is_subagent|caller_type|reviewer_id|parent_id|subagent_type|caller_identity|session_owner|session_token|capability_token|authority_token`)
  — zero matches in `crates/codex1/src/**/*.rs`. No Rust predicate
  branches on caller identity.
- `ParsedPlan` (`crates/codex1/src/cli/plan/parsed.rs:10-22`) has no
  `waves` field; `plan scaffold` / `init` templates
  (`crates/codex1/src/cli/plan/scaffold.rs`,
  `crates/codex1/src/cli/init.rs:125-153`) emit no `waves:` key;
  `MissionState` (`crates/codex1/src/state/schema.rs:177-194`) has no
  `waves` field.
- `TaskRecord` (`crates/codex1/src/state/schema.rs:99-111`) carries only
  `id/status/started_at/finished_at/proof_path/superseded_by` — no lane
  state, no lock token, no reviewer/identity field.
- Reviewer writeback forbidden by skill prompts and CLI surface: only
  `codex1 review record` and `codex1 close record-review` transition
  review state, both main-thread-invoked; skills say so explicitly.
- No wrapper runtime: zero `Command::new` / `tokio::spawn` /
  `daemon` / `spawn_process` in `crates/codex1/src`.
- Ralph hook (`scripts/ralph-stop-hook.sh:25`) runs exactly one command,
  `codex1 status --json`. Matches `01-product-flow.md:229-244`.

Structural invariants (each verified at current HEAD):

- **6 skills + SKILL.md + `agents/openai.yaml`.** All six skills exist
  at `.codex/skills/{autopilot,clarify,close,execute,plan,review-loop}/`;
  every one has both `SKILL.md` and `agents/openai.yaml`. Matches
  `01-product-flow.md:7-17`.
- **Mission files visible under `PLANS/<mission-id>/`.**
  `crates/codex1/src/cli/init.rs:44-46` creates the mission scaffold
  through `state::init_write` + `write_outcome_template` +
  `write_plan_template`. Layout matches `03-planning-artifacts.md:9-26`:
  `OUTCOME.md`, `PLAN.yaml`, `STATE.json`, `EVENTS.jsonl`, `specs/`,
  `reviews/`. `CLOSEOUT.md` is written by
  `crates/codex1/src/cli/close/complete.rs:81-82` on terminal close.
- **Atomic-write protocol.**
  `crates/codex1/src/state/fs_atomic.rs:22-37` follows
  tempfile-in-dir → `sync_data` → `persist` → parent-dir `sync_all`,
  matching `02-cli-contract.md:385-391` ("write state atomically").
  `crates/codex1/src/state/mod.rs:101-152` wraps this under an exclusive
  `fs2` lock, reads state, checks revision, runs the closure, bumps
  `revision` + `events_cursor`, appends EVENTS.jsonl first, then
  atomic-writes STATE.json, then unlocks. Ordering rationale documented
  inline at `mod.rs:126-141`.
- **Mission resolution precedence** matches handoff ordering:
  `crates/codex1/src/core/mission.rs:28-51` — `--mission + --repo-root`
  → `--mission` alone → `--repo-root` alone → CWD walk-up
  (`walk_up_for_mission` / `discover_single_mission`).
- **Verdict derivation matches handoff ordering.**
  `crates/codex1/src/state/readiness.rs:40-66`:
  `TerminalComplete` → `NeedsUser(!outcome.ratified)` → `NeedsUser(!plan.locked)`
  → `Blocked(replan.triggered)` → `Blocked(dirty review)` → mission-close
  family (`ReadyForMissionCloseReview` / `MissionCloseReviewOpen` /
  `MissionCloseReviewPassed`) when `tasks_complete` → `ContinueRequired`.
  Shared by `status` (`cli/status/project.rs:19`) and
  `close check` (`cli/close/check.rs:47`) — identity enforced by
  `ReadinessReport::from_state`. Satisfies `02-cli-contract.md:208`
  ("status and close check must share readiness logic") and suggested
  vocabulary at `02-cli-contract.md:212-221` /
  `00-why-and-lessons.md:166-176`.
- **EVENTS.jsonl append-only + monotonic seq.**
  `crates/codex1/src/state/events.rs:41-47` opens with
  `OpenOptions::new().create(true).append(true).open(path)`; `seq =
  state.events_cursor`, which is bumped exactly once per successful
  `state::mutate` call (`state/mod.rs:125`) and never rewound. No code
  path writes historical events.
- **Hard-planning evidence enforced.**
  `crates/codex1/src/cli/plan/check.rs:274-309` rejects `effective:
  hard` plans unless `planning_process.evidence` is non-empty AND at
  least one entry has `kind ∈ {explorer, advisor, plan_review}`
  (`HARD_EVIDENCE_KINDS`, `cli/plan/parsed.rs:112`). Matches
  `02-cli-contract.md:349-350` and `03-planning-artifacts.md:280-287`.

Anti-goal-free mutation surface (handoff `02-cli-contract.md:108-121`):

- CLI asks no caller-identity questions. Every gated mutation rests on
  artifact state (`plan.locked`, `close.terminal_at`, `replan.triggered`,
  review classification).
- No command spawns subagents. The implementation has no
  `std::process::Command` or async runtime.
- No hidden chat state. All durable mission truth lives under
  `PLANS/<mission-id>/`.

All 14 handoff-suggested error codes
(`02-cli-contract.md:520-534`) are present as `CliError` variants with
stable `code()` strings: `OUTCOME_INCOMPLETE`, `OUTCOME_NOT_RATIFIED`,
`PLAN_INVALID`, `DAG_CYCLE`, `DAG_MISSING_DEP`, `TASK_NOT_READY`,
`PROOF_MISSING`, `REVIEW_FINDINGS_BLOCK`, `REPLAN_REQUIRED`,
`CLOSE_NOT_READY`, `STATE_CORRUPT`, `REVISION_CONFLICT`,
`STALE_REVIEW_RECORD`, `TERMINAL_ALREADY_COMPLETE`
(`crates/codex1/src/core/error.rs:83-100`). The code set is also
reserved with unit coverage per round-1 test-adequacy P1-1/P1-2.

Skill model matrix aligned with handoff (`04-roles-models-prompts.md:21-22,163-167`,
`05-build-prompt.md:97-100`): `.codex/skills/review-loop/SKILL.md:55-59`
lists `code_bug_correctness → gpt-5.3-codex high`,
`local_spec_intent → gpt-5.4 high`, `integration_intent → gpt-5.4 high`,
`plan_quality → gpt-5.4 high|xhigh`, `mission_close → gpt-5.4 high (2
lanes)`. No drift.

## Round-3 spot checks

### Spot check 1 — `cli/plan/check.rs` clears `state.replan.triggered` at relock

Citation: round-2 e2e P0-1 fix committed at
`crates/codex1/src/cli/plan/check.rs:125-134`. Round-2 audit cited
`05-build-prompt.md:87` ("six consecutive dirty reviews trigger replan;
clean resets the consecutive count"); that line is actually about the
counter, not the trigger flag. The real handoff grounding is the
end-to-end replan loop at `01-product-flow.md:40-47`
(Replan → Waves → Execute) combined with `02-cli-contract.md:466-468`
("New tasks are added by editing PLAN.yaml, not by magic") and
`02-cli-contract.md:338-350` (`plan check` is the plan-lock gate). The
flow requires `replan record` → user edits PLAN.yaml → `plan check`
relocks → mission resumes. If `replan.triggered` were not cleared at
relock, `status.verdict` would stay `Blocked` and `close
check`/`close complete`/`close record-review` would refuse to advance
even though the user re-authored the plan the replan asked for. The
round-2 fix matches handoff semantics end-to-end. **Honored.**

### Spot check 2 — `state::require_plan_locked` re-check inside the mutate closure

Citation: round-2 correctness P1-1 fix. The re-check lands at
`crates/codex1/src/cli/task/start.rs:111`,
`crates/codex1/src/cli/task/finish.rs:112`,
`crates/codex1/src/cli/review/start.rs:95`, and
`crates/codex1/src/cli/review/record.rs:185-187` (terminal bypass so
the classifier-first precedence still wins; see round-1 pattern). This
closes a real TOCTOU: the pre-mutate load takes a shared lock, drops
it, then `state::mutate` acquires an exclusive lock; a concurrent
`replan record --supersedes T… --reason six_dirty` could land in
between and leave the mission in `!plan.locked && task.status ==
InProgress`, the exact "task attached to a superseded spec" shape
`state::require_plan_locked` is designed to prevent
(`crates/codex1/src/state/mod.rs:55-71` rationale). The re-check is
the same precondition evaluated under the mutation lock — not a new
gate. It does not over-gate legitimate commands because every
legitimate work-phase mutation is already predicated on `plan.locked`
(confirmed by the pre-mutate calls at each site). **Endorsed by
handoff `02-cli-contract.md:385-391` mutation protocol + `01-product-flow.md`
lock-before-execute ordering. Honored.**

### Spot check 3 — `cli/task/next.rs` short-circuits on `!plan.locked` and `replan.triggered`

Citation: round-2 e2e P2-1 fix at
`crates/codex1/src/cli/task/next.rs:26-53`. On `!state.plan.locked`,
emits `next.kind = "plan"` with hint "Draft and lock PLAN.yaml.". On
`state.replan.triggered`, emits `next.kind = "replan"` with the
recorded reason. This mirrors `cli/status/project.rs::derive_next_action`
at `cli/status/project.rs:144-157`. Both are readiness endpoints
(`task next` is listed at `02-cli-contract.md:365` as "returns the
next ready task or wave"), and the autopilot flow at
`01-product-flow.md:56-82` dispatches on `kind: plan` / `kind: replan`
emitted from `codex1 status --json`. Emitting the same `kind` values
from `task next` is necessary for the two readiness surfaces to agree
(the handoff's shared-readiness invariant at
`02-cli-contract.md:208` was stated for `status` / `close check` but
generalizes to any readiness endpoint a skill may consult). **Aligned
with handoff `task next` contract. Honored.**

## Round-1/round-2 fix verifications (no regressions)

All prior fixes remain intact at HEAD:

- `state::require_plan_locked` pre-mutate call sites
  (`cli/task/start.rs:22`, `cli/task/finish.rs:21`, `cli/review/start.rs:41`,
  `cli/review/record.rs:77`) — present.
- EVENTS-before-STATE write order
  (`cli/state/mod.rs:142-145`) — present, rationale intact.
- Parent-dir `sync_all` (`state/fs_atomic.rs:33-35`) — present.
- `outcome ratify` atomicity (state-first, then OUTCOME.md write) —
  preserved.
- `--expect-revision` enforcement on every short-circuit: `task/start.rs:49,78`,
  `task/finish.rs:74`, `plan/check.rs:78`, `plan/choose_level.rs:53`,
  `review/start.rs:66`, `review/record.rs:101`, `close/complete.rs:53`,
  `close/record_review.rs:95,154`, `outcome/ratify.rs:34`,
  `loop_/mod.rs:75,79,92`.
- Six-consecutive-dirty replan trigger threshold `= 6`:
  `cli/review/record.rs:29`, `cli/close/record_review.rs:26`.
- `review start` dry-run `check_expected_revision`
  (`cli/review/start.rs:66`) — present.
- Skill-matrix alignment
  (`.codex/skills/review-loop/SKILL.md:55-59`) — aligned.
- `plan/dag-quality.md:46` replan-record snippet carries `--reason`
  (round-2 skills P1-1) — present.
- `plan.locked` flat key in status projection
  (`cli/status/project.rs:72-73`) — emitted, matching the skill prose
  fixed in round-2 skills P2-1.

## P0

None.

## P1

None.

## P2

None.

## P3

None.
