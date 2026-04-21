---
name: clarify
description: >
  Ratify OUTCOME.md for a Codex1 mission. Use when the user describes a mission goal but OUTCOME.md is draft, missing fields, or contains fill markers. Interview just enough to fill every required field documented in references/outcome-shape.md, then run `codex1 outcome check` and `codex1 outcome ratify`. Do not start planning or execution from this skill — hand off to $plan after ratification.
---

# Clarify

## Overview

Turn a user's mission goal into a ratified `OUTCOME.md` another Codex thread can act on without hidden chat context. Interview, fill every required field, ratify via the CLI. Do not plan, scaffold, or execute — hand off to `$plan` once ratified.

## Preconditions

A draft `PLANS/<mission-id>/OUTCOME.md` must exist. If it does not, run `codex1 init --mission <id>` first.

## Required workflow

1. **Check phase.** Run `codex1 status --json --mission <id>`. Confirm `verdict == needs_user` and `phase == clarify`. If not, stop and report why.

2. **Read the draft.** Open `PLANS/<mission-id>/OUTCOME.md`. Identify every `[codex1-fill:…]` marker and every empty required field.

3. **Interview the user.** Ask only questions whose answers change the plan. Batch related questions.

   Ask when:
   - The destination can be interpreted multiple ways.
   - Success criteria are not testable.
   - Non-goals are missing for broad work.
   - Constraints are implied but not explicit.
   - Vague terms appear undefined (e.g. "simple", "perfect", "reliable", "done", "thorough", "not overengineered").
   - Destructive actions, deploys, migrations, secrets, money, or external systems are involved.

   Otherwise infer and record the inference in `resolved_questions`.

4. **Write the complete OUTCOME.md.** Fill every required field (see `references/outcome-shape.md`). Leave no fill markers. Use concrete, testable success criteria; consult the bad-vs-good examples in `docs/codex1-rebuild-handoff/03-planning-artifacts.md` if criteria feel vague.

5. **Check.** Run `codex1 --json outcome check --mission <id>`. On `ok:false`, repair the reported `context.missing_fields` and `context.placeholders`, then re-check. Loop until `data.ratifiable == true`.

6. **Ratify.** Run `codex1 --json outcome ratify --mission <id>`. Expect `ok:true` with `data.ratified_at` set.

7. **Hand off.** Suggest `$plan choose-level` and let the main thread pick `light`, `medium`, or `hard`. Do not start planning from here.

## Ratification rule

> No fill markers. No empty required fields. No boilerplate placeholders. No vague "works well" style success criteria.

## Do not include in OUTCOME.md

- `approval_boundaries`
- `autonomy`

These are global workflow/safety rules, not mission destination truth.

## Failure mode

If `codex1 outcome ratify` returns `OUTCOME_INCOMPLETE`, its `context.missing_fields` and `context.placeholders` arrays name exactly what to fix. Repair, re-run `outcome check`, then ratify again.

## Resources

- `references/outcome-shape.md` — one-line intent per required field.
