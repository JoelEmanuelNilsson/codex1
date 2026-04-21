# Round 5 — skills-audit

HEAD audited: `b08d461`.

Scope: six skills at `/Users/joel/codex1/.codex/skills/{clarify,plan,execute,review-loop,close,autopilot}/`.

## Summary

All six skills pass `quick_validate.py` (run in this round; exits `Skill is valid!` for each). Frontmatters carry only `name` + `description`; every description contains a trigger phrase ("Use when" / "Use after"). SKILL.md body lengths are well under the 500-line cap — clarify 60, plan 205, execute 122, review-loop 87, close 112, autopilot 134. No stray `README.md` sits alongside any `SKILL.md`. Every `agents/openai.yaml` exists, sets `allow_implicit_invocation: true`, and carries a `default_prompt` that references the skill's own `$<name>` trigger (`$clarify`, `$plan`, `$execute`, `$review-loop`, `$close`, `$autopilot`). Every `codex1 <verb>` invocation appearing in any skill file or reference resolves to a real clap variant in `crates/codex1/src/cli/mod.rs` + the per-module `*Cmd` enums (full cross-check below).

All prior-round skills fixes remain in place and unregressed:

- `execute/SKILL.md` Step 1 dispatches on `data.next_action.kind` from `codex1 --json status`, with `repair` targets listed under `data.next_action.task_ids` (round-1 skills P1-1).
- `plan/SKILL.md:190` and `plan/references/dag-quality.md:46` both carry the `codex1 replan record --reason <code> --supersedes <id>` form with the `ALLOWED_REASONS` pointer (round-1 skills P1-2 + round-2 skills P1-1).
- `review-loop/SKILL.md` reviewer matrix and `references/reviewer-profiles.md` pin `gpt-5.3-codex` for `code_bug_correctness` and `gpt-5.4` for `local_spec_intent` / `integration_intent` / `plan_quality` / `mission_close`, matching the frozen handoff (round-1 skills P1-3).
- `autopilot/SKILL.md:63` dispatch table contains `fix_state → Escalate to user; do not auto-fix STATE.json.` and `autopilot/references/autopilot-state-machine.md:24` mirrors it (round-1 skills P2-2).
- `plan/SKILL.md:20` reads `outcome_ratified: true` and `execute/SKILL.md:16` reads `plan_locked: true` (flat envelope), matching `cli/status/project.rs:72-73` (round-2 skills P2-1).
- `review-loop/SKILL.md:39-41` accepts both `ready_for_mission_close_review` and `mission_close_review_open` as valid entry verdicts for the mission-close workflow (round-3 skills P1-1).

### CLI verb cross-check

Every `codex1 <verb>` in skills matches a real variant:

- `init --mission <id>` → `Commands::Init` (`cli/init.rs`).
- `status`, `--json status` → `Commands::Status` (`cli/status/mod.rs`).
- `outcome check` / `outcome ratify` → `OutcomeCmd::{Check,Ratify}` (`cli/outcome/mod.rs`).
- `plan choose-level --level <…>` / `plan scaffold --level <…>` / `plan check` / `plan graph --format mermaid` / `plan waves` → `PlanCmd::{ChooseLevel,Scaffold,Check,Graph,Waves}` (`cli/plan/mod.rs`).
- `task next` / `task start T<id>` / `task finish T<id> --proof <path>` / `task packet T<id>` → `TaskCmd::{Next,Start,Finish,Packet}` (`cli/task/mod.rs`).
- `review start T<id>` / `review packet T<id>` / `review record T<id> --clean --reviewers <csv>` / `review record T<id> --findings-file <path> --reviewers <csv>` → `ReviewCmd::{Start,Packet,Record}` (`cli/review/mod.rs`).
- `replan record --reason <code> --supersedes <id>` → `ReplanCmd::Record` (`cli/replan/mod.rs`).
- `loop pause` / `loop resume` / `loop deactivate` → `LoopCmd::{Pause,Resume,Deactivate}` (`cli/loop_/mod.rs`).
- `close check` / `close complete` / `close record-review --clean` / `close record-review --findings-file <path>` → `CloseCmd::{Check,Complete,RecordReview}` (`cli/close/mod.rs`).

Global flags (`--mission`, `--repo-root`, `--json`, `--dry-run`, `--expect-revision`) are declared on the top-level `Cli` struct at `cli/mod.rs:43-65`, so the `--mission <id>` suffix on sub-commands in skills resolves correctly.

### Error code cross-check

Every error code named in skill prose resolves to a real `CliError` variant in `crates/codex1/src/core/error.rs`:

- `OUTCOME_INCOMPLETE` (clarify/SKILL.md) → `CliError::OutcomeIncomplete`.
- `CLOSE_NOT_READY` (close/SKILL.md) → `CliError::CloseNotReady`.
- `TERMINAL_ALREADY_COMPLETE` (close/SKILL.md) → `CliError::TerminalAlreadyComplete`.
- `PLAN_INVALID` / `DAG_CYCLE` / `DAG_MISSING_DEP` (plan/SKILL.md) → `CliError::{PlanInvalid,DagCycle,DagMissingDep}`.
- `REVISION_CONFLICT` / `REPLAN_REQUIRED` / `REVIEW_FINDINGS_BLOCK` / `PROOF_MISSING` / `TASK_NOT_READY` (execute/SKILL.md) — all present as variants.

### Reference files sweep

Reference files checked for stale CLI examples:

- `autopilot/references/autopilot-state-machine.md` — dispatch table, pseudocode loop, and pause-on-close handshake. Every `codex1 <verb>` reference resolves.
- `clarify/references/outcome-shape.md` — table of required OUTCOME fields; no CLI invocations.
- `execute/references/worker-packet-template.md` — packet substitution template; references `codex1 --json task packet T<ID>` only.
- `plan/references/dag-quality.md` — `plan check`, `plan graph --format mermaid`, `plan waves`, `replan record --reason <code> --supersedes <id>` all resolve.
- `plan/references/hard-planning-evidence.md` — spawn templates; only non-mutating reference is to `codex1 --json plan check`.
- `review-loop/references/reviewer-profiles.md` — standing instructions explicitly prohibit mutating CLI; profile table models match the handoff.

No bypass of visible state. No `.ralph/`, `codex1 internal`, or other back-channel mutation paths appear in any skill file (the sole `.ralph/` reference at `close/SKILL.md:112` is a prohibition, not a usage). Skills never instruct agents to write `STATE.json`, `EVENTS.jsonl`, `PLAN.yaml`, or `OUTCOME.md` directly; every surface of those files in skill prose is a prohibition.

## P0

None.

## P1

None.

## P2

None.

## P3

None.
