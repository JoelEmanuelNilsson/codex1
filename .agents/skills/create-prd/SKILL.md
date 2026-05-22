---
name: create-prd
description: Synthesize known context into a local Codex1 PRD artifact. Use when the user wants a PRD from the current conversation, clarification output, repo context, and references.
---

This skill takes the current conversation context and codebase understanding and produces a PRD. Do NOT interview the user — just synthesize what you already know.

Write `PRD.md` as a Codex1 mission artifact.

## Process

1. Explore the repo to understand the current state of the codebase, if you haven't already. Use the project's domain glossary vocabulary throughout the PRD, and respect any ADRs in the area you're touching. If Codex1 workflow docs or mission artifacts exist, use them as local context.

2. Sketch out the major modules you will need to build or modify to complete the implementation. Actively look for opportunities to extract deep modules that can be tested in isolation.

A deep module (as opposed to a shallow module) is one which encapsulates a lot of functionality in a simple, testable interface which rarely changes.

Capture the likely modules and test seams in the PRD. Capture observable success criteria and mission boundaries from known context. If a product, scope, UX, credential, or human-judgment decision is truly missing and the user is actively collaborating, ask. Otherwise record the assumption or unresolved question instead of restarting clarification.

3. Write the PRD using [PRD-FORMAT.md](./PRD-FORMAT.md), then write it as `PRD.md` in the Codex1 mission artifact tree. Keep the PRD at product/outcome level. Do not turn it into a task graph, dependency tracker, priority tracker, per-story acceptance-criteria engine, or execution plan; `$plan` owns execution design.

<prd-template>

## Problem Statement

The problem that the user is facing, from the user's perspective.

## Solution

The solution to the problem, from the user's perspective.

## User Stories

A long numbered list of behavior-focused user stories. Each story should describe one coherent behavior or outcome, not a vague bundle. Use the format:

1. As an <actor>, I want <feature>, so that <benefit>.

<user-story-example>
1. As a mobile bank customer, I want to see balances for each of my accounts, so that I can make better informed decisions about my spending.
</user-story-example>

This list should cover major behavior, actors, edge cases, and artifact interactions, while staying specific enough that `$plan` can map slices back to the stories.

## Success Criteria

Observable, measurable outcomes that make the PRD satisfied. These are mission-level success facts, not implementation tasks or slice-level acceptance criteria.

## Boundaries

### Always Preserve

Existing behaviors, contracts, data, user expectations, workflow boundaries, or artifacts that must remain intact.

### Ask Before Changing

Areas that require explicit human approval before the implementation changes them.

### Out Of Scope

Work this PRD intentionally does not include.

## Module Sketch

Likely modules, interfaces, contracts, and deep-module opportunities. Use stable names and concepts, not brittle paths.

## Implementation Decisions

A list of implementation decisions that were made. This can include:

- The modules that will be built/modified
- The interfaces of those modules that will be modified
- Technical clarifications from the developer
- Architectural decisions
- Schema changes
- API contracts
- State ownership
- Specific interactions

Do NOT include specific file paths or code snippets. They may end up being outdated very quickly.

Exception: if a prototype produced a snippet that encodes a decision more precisely than prose can (state machine, reducer, schema, type shape), inline it within the relevant decision and note briefly that it came from a prototype. Trim to the decision-rich parts — not a working demo, just the important bits.

## Testing Decisions

A list of testing decisions that were made. Include:

- A description of what makes a good test (only test external behavior, not implementation details)
- Which modules will be tested
- Prior art for the tests (i.e. similar types of tests in the codebase)
- Testing non-goals

## Proof Expectations

Commands, tests, screenshots, manual checks, review evidence, or other proof expected later.

## Review Expectations

Reviewer posture, review artifacts, triage expectations, or explicit "no special review" statement.

## Further Notes

Any further notes about the feature.

</prd-template>
