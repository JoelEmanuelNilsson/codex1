# Round 2 — skills-audit

## Summary

Audited the six skills under `/Users/joel/codex1/.codex/skills/` (clarify, plan, execute, review-loop, close, autopilot) against round-1 decisions, `cli-creator` / `skill-creator` conventions, and the frozen handoff.

Verification pass — all holding from round 1:

- `quick_validate.py` passes for every skill (clarify, plan, execute, review-loop, close, autopilot).
- Frontmatter is `name` + `description` only; every description carries a trigger phrase ("Use when…" / "Use after…"); all SKILL.md bodies are ≤ 500 lines (longest is `plan/SKILL.md` at 205 lines). No `README.md` files alongside SKILL.md.
- Every `agents/openai.yaml` exists with `default_prompt` referencing `$<skill-name>`.
- Every `codex1 <verb>` referenced in a skill resolves to a real subcommand in `crates/codex1/src/cli/mod.rs` (init, doctor, hook, outcome {check, ratify}, plan {choose-level, scaffold, check, graph, waves}, task {next, start, finish, status, packet}, review {start, packet, record, status}, replan {check, record}, loop {activate, pause, resume, deactivate}, close {check, complete, record-review}, status). No `codex1 internal <verb>` references remain.
- No skill writes `STATE.json` / `EVENTS.jsonl` / `PLAN.yaml` / `OUTCOME.md` directly — all mentions are prohibitions. No `.ralph/` bypass references (the one hit is a prohibition in `close/SKILL.md:112`).
- Round-1 skills fixes confirmed in place:
  - `execute/SKILL.md:26-42` — Step 1 reads `codex1 --json status` and dispatches on `data.next_action.kind` (including `repair` and `replan` kinds with the `task_ids` callout), per the handoff single-source rule.
  - `plan/SKILL.md:190` — replan snippet is `codex1 replan record --reason <code> --supersedes <id>` and points at `crates/codex1/src/cli/replan/triggers.rs::ALLOWED_REASONS`.
  - `review-loop/SKILL.md:53-59` model table and `review-loop/references/reviewer-profiles.md:68,88,107,126,146` reviewer profiles use `gpt-5.3-codex` (code_bug_correctness) and `gpt-5.4` (local_spec_intent / integration_intent / plan_quality / mission_close), matching `docs/codex1-rebuild-handoff/04-roles-models-prompts.md:21-24,163-167`.
  - `autopilot/SKILL.md:63` dispatch table includes `fix_state → Escalate to user; do not auto-fix STATE.json.`, and `autopilot/references/autopilot-state-machine.md:24` mirrors it.

Two new findings below: one P1 (a round-1 fix that did not reach the `plan/` reference file and would cause a CLI parse failure for any agent following it) and one P2 (two SKILL prose lines that attribute nested STATE paths to the flat `codex1 status --json` envelope).

## P0

(none)

## P1

### P1-1 · `plan/references/dag-quality.md:46` prescribes `codex1 replan record` without the required `--reason`

File: `/Users/joel/codex1/.codex/skills/plan/references/dag-quality.md:46`

Current text:

> Use `codex1 replan record --supersedes <id>` for tasks being abandoned.

The CLI defines `reason: String` (not `Option<String>`) on `ReplanCmd::Record` at `/Users/joel/codex1/crates/codex1/src/cli/replan/mod.rs:22-30`, so `--reason <code>` is a required argument — omitting it produces a clap parse error and the command exits non-zero before any state mutation. Round-1 skills P1-2 corrected the main `plan/SKILL.md:190` line to `codex1 replan record --reason <code> --supersedes <id>`, but the parallel instruction in `references/dag-quality.md` was not updated and still directs an agent to a call that cannot succeed.

Reproducible bug (category b): an agent loading `dag-quality.md` and copying line 46 verbatim will hit a CLI failure on first invocation. Fix is a text-only change: update the line to match the `plan/SKILL.md:190` form (`codex1 replan record --reason <code> --supersedes <id>`, with a pointer to `crates/codex1/src/cli/replan/triggers.rs::ALLOWED_REASONS`).

## P2

### P2-1 · `plan/SKILL.md:20` and `execute/SKILL.md:16` attribute nested STATE paths to the flat `codex1 status --json` envelope

Files:

- `/Users/joel/codex1/.codex/skills/plan/SKILL.md:20` — "`codex1 --json status` shows `outcome.ratified: true` (verdict is not `needs_user` for the outcome)."
- `/Users/joel/codex1/.codex/skills/execute/SKILL.md:16` — "`PLAN.yaml` is locked. `codex1 --json status` must show `plan.locked: true` and `verdict: continue_required`."

The actual `codex1 status --json` envelope publishes flat top-level fields `outcome_ratified` and `plan_locked` (not nested `outcome.ratified` / `plan.locked`). This is locked in at `/Users/joel/codex1/crates/codex1/src/cli/status/project.rs:72-73`:

```rust
"outcome_ratified": state.outcome.ratified,
"plan_locked": state.plan.locked,
```

Empirical confirmation against the release binary on a fresh mission (`codex1 init --mission test-audit` then `codex1 --json status --mission test-audit`) emits `"outcome_ratified": false` / `"plan_locked": false` — no nested `outcome` or `plan` objects in `data`.

Reproducible bug (category b): an agent that reads either SKILL precondition literally and looks up `status.data.outcome.ratified` or `status.data.plan.locked` will find nothing and may mis-conclude the precondition fails. The nested `outcome.ratified` / `plan.locked` paths do exist on STATE.json (`src/state/schema.rs:56-73`), but they are not the envelope shape the skills instruct callers to read. Fix is text-only: replace `outcome.ratified` → `outcome_ratified` and `plan.locked` → `plan_locked` in these two prose lines.

The parallel lines in `close/SKILL.md:24-27` describe abstract completion requirements with STATE-path references in parentheses; they do not attribute the paths to the `status` envelope and are defensible as STATE-shape references. Not flagged.

## P3

(none)
