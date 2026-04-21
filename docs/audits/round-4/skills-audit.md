# Round 4 — skills-audit

HEAD audited: `703a171`.

Scope: six skills at `/Users/joel/codex1/.codex/skills/{clarify,plan,execute,review-loop,close,autopilot}/`.

## Summary

All six skills pass `quick_validate.py`. Frontmatters contain only `name` + `description`. Body lengths: clarify 61, plan 206, execute 123, review-loop 88, close 113, autopilot 135 — all well under the 500-line cap. No stray `README.md` alongside any `SKILL.md`. Every `agents/openai.yaml` exists, carries a `default_prompt` string that references the skill's own `$<name>` trigger, and sets `allow_implicit_invocation: true`. Every `codex1 <verb>` invocation in any skill file or reference file resolves to a real clap variant in `crates/codex1/src/cli/` (full cross-check enumerated below).

Round-1 through round-3 fixes all still hold:

- `review-loop/SKILL.md` mission-close step 1 dispatches on both `ready_for_mission_close_review` (first round) and `mission_close_review_open` (re-entry after a dirty mission-close record). Round-3 skills P1-1 fix is intact at lines 39–41.
- `execute/SKILL.md` Step 1 dispatches on `data.next_action.kind` from `codex1 --json status`, with `repair` listing targets under `data.next_action.task_ids` (round-1 skills P1-1).
- `plan/SKILL.md` replan snippet carries `--reason <code>` with a pointer to `ALLOWED_REASONS` (round-1 skills P1-2). `plan/references/dag-quality.md:46` carries the same corrected form (round-2 skills P1-1).
- `review-loop/SKILL.md` reviewer matrix pins `code_bug_correctness → gpt-5.3-codex` and `local_spec_intent → gpt-5.4` per the frozen handoff (round-1 skills P1-3).
- `autopilot/SKILL.md` dispatch table contains the `fix_state → Escalate to user; do not auto-fix STATE.json` row (round-1 skills P2-2).
- `plan/SKILL.md:20` references `outcome_ratified: true` (flat envelope path) and `execute/SKILL.md:16` references `plan_locked: true` (flat envelope path) — round-2 skills P2-1 fix still intact.

### CLI verb cross-check

Every `codex1 <verb>` in skills matches a real variant in `crates/codex1/src/cli/`:

- `init --mission <id>` → `Commands::Init` (`cli/init.rs`).
- `status`, `--json status` → `Commands::Status` (`cli/status/mod.rs`).
- `outcome check` / `outcome ratify` → `OutcomeCmd::{Check,Ratify}` (`cli/outcome/mod.rs`).
- `plan choose-level --level <…>` / `plan scaffold --level <…>` / `plan check` / `plan graph --format mermaid` / `plan waves` → `PlanCmd::{ChooseLevel, Scaffold, Check, Graph, Waves}` (`cli/plan/mod.rs`).
- `task next` / `task start T<id>` / `task finish T<id> --proof <path>` / `task packet T<id>` → `TaskCmd::{Next, Start, Finish, Packet}` (`cli/task/mod.rs`).
- `review start T<id>` / `review packet T<id>` / `review record T<id> --clean --reviewers <csv>` / `review record T<id> --findings-file <path> --reviewers <csv>` → `ReviewCmd::{Start, Packet, Record}` (`cli/review/mod.rs`).
- `replan record --reason <code> --supersedes <id>` → `ReplanCmd::Record` (`cli/replan/mod.rs`).
- `loop pause` / `loop resume` / `loop deactivate` → `LoopCmd::{Pause, Resume, Deactivate}` (`cli/loop_/mod.rs`).
- `close check` / `close complete` / `close record-review --clean` / `close record-review --findings-file <path>` → `CloseCmd::{Check, Complete, RecordReview}` (`cli/close/mod.rs`).

Global flags (`--mission`, `--repo-root`, `--json`, `--dry-run`, `--expect-revision`) are declared on top-level `Cli` in `cli/mod.rs:43-65`, so every sub-command's `--mission <id>` usage in the skills resolves correctly.

### Envelope shape cross-check (spot checks on fields named in skill prose)

- `data.ratifiable` — `cli/outcome/check.rs:36-39`. Matches `clarify/SKILL.md:37`.
- `data.ratified_at` — `cli/outcome/ratify.rs` sets `state.outcome.ratified_at`; the ratify envelope surfaces it in `data`. Matches `clarify/SKILL.md:39`.
- `data.review_profile`, `profiles`, `targets`, `diffs`, `proofs`, `mission_summary` — `cli/review/packet.rs:66-77`. Matches `review-loop/SKILL.md:21`.
- `data.replan_triggered` — `cli/review/record.rs:222`. Matches `review-loop/SKILL.md:35`.
- `data.ready`, `data.verdict` on `close check` — `cli/close/check.rs:160-165`. Matches `close/SKILL.md:28` and `review-loop/SKILL.md:51`.
- `data.next_action.kind`, `data.next_action.task_ids` (on `repair`) — `cli/status/project.rs:158-164`. Matches `execute/SKILL.md:39` and the autopilot dispatch table at `autopilot/SKILL.md:52-64`.
- Verdict strings (`continue_required`, `needs_user`, `blocked`, `ready_for_mission_close_review`, `mission_close_review_open`, `mission_close_review_passed`, `terminal_complete`, `invalid_state`) — `state/readiness.rs:23-34`. All skill references match.
- Phase string `clarify` — `state/schema.rs:19-26`. Matches `clarify/SKILL.md:19` prose `phase == clarify`.

No bypass of visible state found. No `.ralph/`, `codex1 internal`, or other back-channel mutation paths appear in any skill file.

## P0

None.

## P1

None.

## P2

None.

## P3

- **execute/SKILL.md:32 claims `task next` "cannot surface `repair` or `replan`".** The "cannot surface `repair`" half is correct (`cli/task/next.rs:58-97` emits `plan | replan | mission_close_review | run_review | blocked | run_task | run_wave` but never `repair`), but the "or `replan`" half is contradicted by `cli/task/next.rs:40-52`, which explicitly emits `{next: {kind: "replan", reason: …}}` when `state.replan.triggered`. Framing: this is CLI+skill factual drift in descriptive prose, not in the dispatch path. The skill's Step 1 dispatches on `data.next_action.kind` from `codex1 --json status` (round-1 skills P1-1 fix); the `task next` references at lines 24, 78, and 107 are supplementary prose that informs the loop-termination shape but does not drive dispatch. No agent following the Step 1→5 workflow ever inspects a `task next` kind to decide what to do, so an agent cannot be led astray by this drift. Fails strict validity bar (a/b/c) — belongs at P3 by the round-1 precedent on "execute SKILL 'Use after' wording" and the round-2/3 precedent on non-loop-scope prose drift. Surfacing only for completeness; recommend REJECT.
