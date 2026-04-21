# Round 1 — skills-audit

## Summary

All six skills under `/Users/joel/codex1/.codex/skills/` (clarify, plan, execute, review-loop, close, autopilot) pass `quick_validate.py`. Frontmatter is minimal (`name` + `description` only), bodies are ≤ 500 lines, every `default_prompt` references `$<skill-name>`, no `README.md` stubs exist alongside, no skill instructs agents to edit `.ralph/`, `STATE.json`, or `EVENTS.jsonl` directly, and no skill references the removed `codex1 internal` subcommand set. Every `codex1 <verb> <subverb>` referenced across the skills corresponds to a real variant in `crates/codex1/src/cli/mod.rs` + the per-subcommand `*Cmd` enums.

Two reproducible bugs break the "written instructions cause an agent to fail" bar:

- **Execute SKILL.md dispatches on kinds (`repair`, `replan`) that `codex1 task next` never emits.** The command in `crates/codex1/src/cli/task/next.rs` has no `state.replan.triggered` branch and no `repair` branch — those kinds live only in `codex1 status --json` via `cli/status/project.rs`. An agent following the skill's decision list misses the replan/repair cases entirely.
- **Plan SKILL.md's replan snippet omits the required `--reason <CODE>` flag,** so `codex1 replan record --supersedes <id>` fails clap before it can run.

A further contract-level divergence: `review-loop/SKILL.md` and `review-loop/references/reviewer-profiles.md` specify `claude-opus-4-7` (and "`claude-opus-4-7` or `gpt-5.4`") for the `code_bug_correctness` and `local_spec_intent` reviewer profiles; the handoff model matrix in `docs/codex1-rebuild-handoff/04-roles-models-prompts.md` specifies `gpt-5.3-codex` for "Code bug/correctness reviewer" and `gpt-5.4` for "Spec/intent reviewer". The skill has silently migrated reviewer defaults to Claude without the handoff agreeing.

No P0 findings.

## P0

(none)

## P1

### P1-1 Execute skill dispatches on task-next kinds the CLI never emits

**Citation.** `.codex/skills/execute/SKILL.md:37-41` vs `crates/codex1/src/cli/task/next.rs:16-71` (and `crates/codex1/src/cli/status/project.rs:138-151` for where those kinds actually live).

**Evidence.** Execute's Step 1 instructs the agent to run `codex1 --json task next` and then "Inspect `data.next.kind`" with this table:

```
- `run_task` …
- `run_wave` …
- `run_review` …
- `mission_close_review` …
- `repair` — run the named repair task (same flow as `run_task`) …
- `replan` — return with: "Replan required. Hand to `$plan replan`."
```

`crates/codex1/src/cli/task/next.rs` only ever emits one of: `mission_close_review` (when all tasks terminal), `run_review`, `run_task`, `run_wave`, or `blocked`. It never reads `state.replan.triggered` and never produces a `"repair"` kind. Those two kinds are produced exclusively by `cli/status/project.rs::derive_next_action` (lines 138-151).

**Consequence.** When a reviewer dirty-count threshold fires (`state.replan.triggered == true`, `state.plan.locked == false`), `task next` returns `{"kind":"blocked", …}`. The skill tells the agent to fall through to "CLI returns `REPLAN_REQUIRED`: stop and surface" — so autopilot/execute never hands to `$plan replan`. Repair is similar: when a review records P0/P1/P2 findings, `task next` does not emit `repair`; only `status` does. Execute running in isolation literally cannot observe repair or replan dispatch.

**Suggested fix.** One of (a) change Step 1 to `codex1 --json status` (the single source of truth the handoff calls out in `01-product-flow.md:151`) and dispatch on `data.next_action.kind`; (b) add `repair`/`replan` branches to `cli/task/next.rs` that mirror `derive_next_action`; or (c) delete the `repair` / `replan` bullets from the Step 1 list and spell out explicitly that the agent must consult `codex1 --json status` to observe those kinds before looping.

### P1-2 Plan skill's `replan record` example omits the required `--reason <CODE>` flag

**Citation.** `.codex/skills/plan/SKILL.md:190` vs `crates/codex1/src/cli/replan/mod.rs:23-30` and `crates/codex1/src/cli/replan/record.rs:26-35`.

**Evidence.** Plan SKILL.md line 190:

```
- Record `codex1 replan record --supersedes <id>` for each task that is being superseded.
```

`ReplanCmd::Record` declares `reason: String` (no default, no `Option`), so clap rejects the invocation without `--reason <CODE>`. Further, `record::run` rejects anything outside `triggers::ALLOWED_REASONS`. An agent pasting the skill snippet hits a `ParseError` on the first call.

**Suggested fix.** Change the line to `codex1 replan record --reason <code> --supersedes <id>` (with a pointer to the allowed reason codes in `cli/replan/triggers.rs`, or the plan skill's own replan section). Optionally call out that `--supersedes` may be passed zero or more times but `--reason` is mandatory.

### P1-3 Review-loop reviewer model table disagrees with handoff model matrix

**Citation.** `.codex/skills/review-loop/SKILL.md:50-57`, `.codex/skills/review-loop/references/reviewer-profiles.md:66-88` vs `docs/codex1-rebuild-handoff/04-roles-models-prompts.md:21-22`.

**Evidence.** Handoff `04-roles-models-prompts.md`:

```
| Code bug/correctness reviewer | `gpt-5.3-codex` | high | Use two lanes for high-risk code |
| Spec/intent reviewer          | `gpt-5.4`        | high | xhigh for hard-plan review        |
```

Skill `review-loop/SKILL.md`:

```
| `code_bug_correctness` | Code-producing or code-heavy repair task | `claude-opus-4-7` high | 1-2 |
| `local_spec_intent`    | One task/spec versus intended behavior   | `claude-opus-4-7` or `gpt-5.4` high | 1 |
```

Skill `review-loop/references/reviewer-profiles.md:68,88` also names `claude-opus-4-7` as the recommended reviewer model. The handoff model matrix never mentions any `claude-*` model for any role.

**Suggested fix.** Either (a) update the skill tables to match the handoff (`gpt-5.3-codex` for `code_bug_correctness`, `gpt-5.4` for `local_spec_intent`, etc.), or (b) update the handoff matrix to add the Claude-family equivalents the project actually intends to use. The two documents must agree on the authoritative model list, because reviewer model choice is a contract, not a style preference.

## P2

### P2-1 Execute skill adds Claude-family "equivalents" not present in the handoff model matrix

**Citation.** `.codex/skills/execute/SKILL.md:84-86` vs `docs/codex1-rebuild-handoff/04-roles-models-prompts.md:9-26`.

**Evidence.** Execute SKILL.md lines 84-86:

```
- Coding worker (code-heavy): `gpt-5.3-codex` at reasoning `high`. Claude-family equivalent: `claude-opus-4-7` for code-heavy work.
- Intent-heavy worker …: `gpt-5.4` at reasoning `high`. Claude-family equivalent: `claude-sonnet-4-6` (a.k.a. gpt-5.4).
- Small mechanical worker / spark-level edits: `gpt-5.3-codex-spark` at reasoning `high`. A haiku-class model is acceptable here.
```

The handoff matrix lists only the gpt-family defaults. The assertion that `claude-sonnet-4-6` is "a.k.a. gpt-5.4" is not in any source document I can find and looks like skill-level extrapolation. Lower severity than P1-3 because the default coding worker model in the skill still matches the handoff (`gpt-5.3-codex`) — the divergence is in the declared equivalents, not the default.

**Suggested fix.** Either remove the "Claude-family equivalent" rows, or add a short section to the handoff that lists them as officially supported substitutes. Drop the "`claude-sonnet-4-6` (a.k.a. gpt-5.4)" alias line regardless; it implies identity where the handoff only describes `gpt-5.4` as its own model.

### P2-2 Autopilot dispatch table undercovers the `fix_state` next-action

**Citation.** `.codex/skills/autopilot/SKILL.md:54-64` and `.codex/skills/autopilot/references/autopilot-state-machine.md:10-24` vs `crates/codex1/src/cli/status/project.rs:112-117`.

**Evidence.** `derive_next_action` emits `{"kind":"fix_state", …}` when `verdict == Verdict::InvalidState`. The autopilot SKILL.md dispatch table (lines 54-64) lists `clarify`, `plan`, `run_task`, `run_wave`, `run_review`, `mission_close_review`, `repair`, `replan`, `close`, `blocked`, `closed` — it omits `fix_state`. The reference state-machine file does include it (line 24 + pseudocode lines 81-83), so the reference saves the day, but a user who only reads SKILL.md is told "Any other kind is an escalation surface" without being told `fix_state` is the specific kind that will appear.

**Suggested fix.** Add one row to the SKILL.md table: `| fix_state | Escalate to user; do not auto-fix. |`. It costs one line and eliminates a gap between SKILL.md and its own reference.

## P3

### P3-1 Execute description opens with "Use after" rather than the skill-creator-preferred "Use when"

**Citation.** `.codex/skills/execute/SKILL.md:4` vs `/Users/joel/.codex/skills/.system/skill-creator/SKILL.md:353-354` ("Include all 'when to use' information here").

**Evidence.** The description starts "Run the next ready task or ready wave … Use after `$plan` has locked PLAN.yaml and before mission close." The skill-creator rule is not strictly a phrase requirement ("Use when …" is a typical trigger only), but the other five skills all open with "Use when …", making execute stylistically inconsistent. Pure style.

**Suggested fix.** Reword to "Use when `$plan` has locked PLAN.yaml and the mission has ready work before mission close." Optional.

### P3-2 Clarify skill's handoff hint mixes skill name and CLI verb

**Citation.** `.codex/skills/clarify/SKILL.md:41`.

**Evidence.** Line 41: "Suggest `$plan choose-level` and let the main thread pick `light`, `medium`, or `hard`." `$plan` is the skill; `choose-level` is a CLI verb the plan skill internally wraps. A reader could misread this as "invoke `$plan choose-level` as a skill-subcommand form." Minor ergonomic confusion.

**Suggested fix.** Rephrase as "Hand off to `$plan` (which will run `codex1 plan choose-level` first)."

### P3-3 Close skill's terminal-close workflow duplicates guidance already in review-loop

**Citation.** `.codex/skills/close/SKILL.md:68-91` and `.codex/skills/review-loop/SKILL.md:37-47`.

**Evidence.** Both skills document running `codex1 close complete` after mission-close review passes. Close explicitly calls this out as a separation ("Separately documents the terminal-close path…" in the frontmatter), but the two copies could drift. Not a bug today; worth flagging as maintenance risk.

**Suggested fix.** Keep the authoritative workflow in one place (close), and have review-loop end with "hand to `$close` terminal-close workflow" rather than re-listing the `close complete` command itself. Optional.
