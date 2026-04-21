---
name: review-loop
description: >
  Run reviewer subagents for a planned review task or mission-close review and record findings through `codex1 review record`. Use when `codex1 task next` reports `kind: run_review` or `kind: mission_close_review`. Spawns one or more reviewer subagents per profile (code_bug_correctness, local_spec_intent, integration_intent, plan_quality, mission_close). Reviewers return NONE or P0/P1/P2 findings only. Main thread records the outcome. Triggers replan at six consecutive dirty reviews for the same active target.
---

# Review Loop

## Overview

`$review-loop` orchestrates reviewer subagents for a planned review task or the mission-close review. Reviewers return findings only (NONE or P0/P1/P2). They do not edit files, invoke Codex1 skills, or run CLI mutations. The main thread is the sole writer of mission truth: it records clean/dirty via `codex1 review record` (planned review) or `codex1 close record-review` (mission close).

## Preconditions

`codex1 task next --json` returned `kind: run_review` or `kind: mission_close_review`. If neither, stop and report status; do not spawn reviewers.

## Required workflow (planned review task)

1. **Start.** Run `codex1 --json review start T<id>` to open the review.

2. **Fetch packet.** Run `codex1 --json review packet T<id>`. Note `data.review_profile` (or `profiles`), `data.targets`, `data.diffs`, `data.proofs`, and `data.mission_summary`.

3. **Spawn reviewers.** Spawn one reviewer subagent per profile (see `references/reviewer-profiles.md` for spawn templates and models). Paste the packet into the prompt. Reviewers must return `NONE` or `P0/P1/P2` findings with evidence refs only — no edits, no CLI, no repair.

4. **Record outcome on main thread.** Collect all reviewer responses.
   - If every reviewer returned `NONE`:
     ```bash
     codex1 --json review record T<id> --clean --reviewers <csv>
     ```
   - If any reviewer returned `P0/P1/P2`: aggregate findings into a markdown file (e.g. `PLANS/<mission-id>/reviews/findings-T<id>-<timestamp>.md`), then:
     ```bash
     codex1 --json review record T<id> --findings-file <path> --reviewers <csv>
     ```

5. **Check replan.** Inspect `data.replan_triggered` on the record response. If true, hand off to `$plan replan`. Otherwise return control to `$execute`.

## Required workflow (mission-close review)

1. **Confirm readiness.** Run `codex1 --json close check`. Require `data.verdict == ready_for_mission_close_review`. If not, stop.

2. **Spawn mission-close reviewers.** Spawn 1-2 reviewers with profiles `mission_close` and (optionally) `integration_intent`. The packet must include `OUTCOME.md`, `PLAN.yaml`, and the final `CLOSEOUT-preview` — see `references/reviewer-profiles.md` for templates.

3. **Record outcome on main thread.**
   - Clean: `codex1 --json close record-review --clean`
   - Dirty: `codex1 --json close record-review --findings-file <path>`

4. **Replan or close.** If mission-close review has gone dirty six times in a row, hand off to `$plan replan`. Otherwise, once clean, re-run `codex1 --json close check`; when `data.ready == true`, hand off to the user or `$close` for `codex1 close complete`.

## Reviewer profiles

Models follow the reviewer matrix in `docs/codex1-rebuild-handoff/04-roles-models-prompts.md`.

| Profile | When | Model | Lanes |
| --- | --- | --- | --- |
| `code_bug_correctness` | Code-producing or code-heavy repair task | `gpt-5.3-codex` high | 1-2 |
| `local_spec_intent` | One task/spec versus intended behavior | `gpt-5.4` high | 1 |
| `integration_intent` | Multi-task / wave / subsystem interaction | `gpt-5.4` high | 1 |
| `plan_quality` | Plan critique before lock (hard plans) | `gpt-5.4` high or xhigh | 1-2 |
| `mission_close` | Final mission-close review | `gpt-5.4` high | 2 for important missions |

See `references/reviewer-profiles.md` for the spawn-prompt template per profile.

## Reviewer standing instructions

Every spawn prompt must begin with the standing reviewer block in `references/reviewer-profiles.md` (findings-only, no edits, no `codex1` mutations, no repairs, no marking clean anywhere). The main thread is the sole writer of mission truth. Reviewer writeback is forbidden.

## Late-output categories

`codex1 review record` classifies each record into one of:

- `accepted_current` — recorded before the review boundary closed. Only this category moves the dirty counter.
- `late_same_boundary` — arrived after current but within the same boundary revision.
- `stale_superseded` — belongs to a superseded task/review boundary.
- `contaminated_after_terminal` — arrived after mission terminal.

`late_same_boundary`, `stale_superseded`, and `contaminated_after_terminal` records are appended to `EVENTS.jsonl` for audit but do not change mission truth. Do not retry a stale record by editing state directly; let the CLI classify.

## Do not

- Edit files on behalf of reviewers.
- Record review results from inside a reviewer subagent — only the main thread records.
- Replan from this skill — hand off to `$plan replan` when the replan trigger fires.
- Close the mission from this skill — `$close` / `codex1 close complete` owns terminal.
