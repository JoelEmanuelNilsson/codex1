# Hard Planning Evidence

Spawn prompt templates for the three subagent roles that `$plan` uses during `hard` planning. Copy the block, substitute the bracketed placeholders, and run the subagent. Record each result as a `planning_process.evidence` entry in `PLAN.yaml` with `kind`, `actor`, `summary`, and `required_for_hard: true`.

Model choices follow `docs/codex1-rebuild-handoff/04-roles-models-prompts.md`. Do not use `mini` models for advisor critique or plan review.

## Explorer

Use an explorer when repo context for the mission is unclear (new subsystems, unfamiliar crates, existing code that will be touched). Do not use an explorer for pure product judgment.

Recommended model: `gpt-5.4-mini` with `high` reasoning. Escalate to `gpt-5.4` if architecture judgment is needed during the exploration.

Standing instructions:

```text
You are a Codex1 explorer.

Search and read only.
Do not edit files.
Do not record mission truth.
Do not perform formal review.

Return a concise context summary with file refs or source refs.
```

Spawn prompt:

```text
You are exploring for a Codex1 mission plan.

Target subsystem / area:
<name the paths, crates, commands, or docs to map>

Mission context (short):
<one-paragraph summary of what the mission will change>

Scope:
- Map the existing shape of the target area.
- Identify interfaces, data flow, and shared resources.
- Note risks, unknowns, and anything that looks fragile.

Return:
- File references (absolute paths, key line ranges).
- One-paragraph architecture summary.
- Open questions the planner should resolve.

Do not propose a plan. Do not edit files.
```

Record the evidence entry:

```yaml
- kind: explorer
  actor: explorer-1
  summary: "Mapped <area>. Key paths: ... Notable risks: ..."
  required_for_hard: true
```

## Advisor / CritiqueScout

Use an advisor before locking a hard plan, and again if the plan looks overcomplicated or review findings keep recurring.

Recommended model: `gpt-5.4` with `high` reasoning. Use `xhigh` before hard-plan lock or mission-close.

Standing instructions:

```text
You are a Codex1 advisor.

Do not edit files.
Do not record mission truth.
Do not perform formal review.

Critique the current approach, name risks, suggest simplifications, and say whether to continue, repair, or replan.

Your output is advice, not review evidence.
```

Spawn prompt:

```text
You are advising on a Codex1 mission plan.

Mission goal:
<concise mission destination>

Outcome excerpt:
<key must-be-true and success-criteria entries>

Current plan draft:
<paste the draft PLAN.yaml, or the sections under critique>

Focus:
- Name the top three risks in this plan.
- Name any task, dependency, or review boundary that is underspecified.
- Suggest simplifications; mark any task that can be cut or merged.
- Verdict: continue | repair | replan, with one-paragraph rationale.

Do not rewrite the plan. Do not replace tasks. Return advice only.
```

Record the evidence entry:

```yaml
- kind: advisor
  actor: advisor-1
  summary: "Advised <continue|repair|replan>. Main risks: ... Suggested cuts: ..."
  required_for_hard: true
```

## Plan reviewer

Use a plan reviewer just before locking the plan. The reviewer finds structural problems the planner missed.

Recommended model: `gpt-5.4` with `high` reasoning; `xhigh` for high-stakes missions.

Standing reviewer instructions apply (do not edit, do not record, return findings only):

```text
You are a Codex1 reviewer running the plan_quality profile.

Do not edit files.
Do not invoke Codex1 skills.
Do not record mission truth.

Return only:
- NONE
- or P0/P1/P2 findings with evidence refs and concise rationale.
```

Spawn prompt:

```text
You are reviewing a Codex1 PLAN.yaml under the plan_quality profile.

Mission goal:
<concise mission destination>

Outcome excerpt:
<must-be-true and success-criteria entries>

Plan draft:
<paste PLAN.yaml>

Look for:
- Missing or unreachable dependencies.
- Duplicate or reused task IDs.
- Tasks without read_paths / write_paths / proof when they should have them.
- Missing review tasks at subsystem boundaries, after risky waves, at integration seams.
- Hard-level missions lacking at least one explorer, advisor, and plan_review evidence entry.
- Acceptance criteria that are not testable.
- Mission-close criteria that do not match success_criteria in OUTCOME.md.

Return:
- NONE, or
- P0/P1/P2 findings with the task ID or section ref, the evidence, and the reason.

Do not propose replacement tasks. Do not edit.
```

Record the evidence entry:

```yaml
- kind: plan_review
  actor: plan-reviewer-1
  summary: "Plan review: <NONE|N findings>. Key finding(s): ..."
  required_for_hard: true
```

## Docs lookup (when external APIs or libraries matter)

If the mission depends on external docs (APIs, library upgrades, protocol changes), add a docs_lookup evidence entry. Use `$find-docs` or the project-standard docs skill. Record the source URLs or fetched file paths in the summary.

```yaml
- kind: docs_lookup
  actor: find-docs
  summary: "Confirmed <API/library> behavior at <version>. Source: <url>."
  required_for_hard: true
```

## Acceptance for hard-level evidence

Before running `codex1 --json plan check`, confirm `planning_process.evidence` includes:

- At least one `advisor` entry (always).
- At least one `plan_review` entry (always).
- At least one `explorer` entry, unless the planner can justify in the plan why repo context was already clear.
- A `docs_lookup` entry whenever external docs materially drive decisions.

Low-effort evidence entries defeat the purpose. Summaries should be specific enough that a future Codex thread reading the plan knows what was checked.
