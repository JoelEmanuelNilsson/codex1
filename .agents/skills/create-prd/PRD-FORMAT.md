# PRD Format

Use this when `$create-prd` writes `PRD.md`.

## Required Quality Bar

The PRD must be sufficient for `$plan` to design execution without reconstructing product intent. Write from the user's perspective first, then capture observable success criteria, mission boundaries, implementation decisions, and testing decisions.

Completion scope default: PRD is the final finished-product contract unless the user asks for staged delivery; put exclusions in boundaries.

## Template

```md
## Problem Statement

The problem the user is facing, from the user's perspective.

## Solution

The solution, from the user's perspective.

## User Stories

A long numbered list of behavior-focused user stories:

1. As an <actor>, I want <feature>, so that <benefit>.

Each story should describe one coherent behavior or outcome. Avoid vague bundles like "manage settings" unless they are split into observable behaviors. Cover all major behavior, actors, edge cases, and artifact interactions.

## Success Criteria

Observable, measurable outcomes that make the PRD satisfied. These are mission-level success facts, not implementation tasks or slice-level acceptance criteria.

## Boundaries

Default boundary: do not introduce fallback paths, legacy compatibility, duplicate sources of truth, or parallel information flows unless the user or existing artifacts explicitly require them.

### Always Preserve

Existing behaviors, contracts, data, user expectations, workflow boundaries, or artifacts that must remain intact.

### Ask Before Changing

Areas that require explicit human approval before the implementation changes them.

### Out Of Scope

Work this PRD intentionally does not include.

## Module Sketch

Likely modules, interfaces, contracts, and deep-module opportunities. Use stable names and concepts, not brittle paths.

## Implementation Decisions

- Modules that will be built or modified
- Interfaces or contracts that change
- Technical clarifications
- Architectural decisions
- Schema changes
- API contracts
- State ownership
- Specific interactions

Do not include brittle file paths or code snippets.

## Testing Decisions

- What makes a good test for this change
- External behavior to test
- Modules worth testing directly
- Prior art in the existing test suite
- Testing non-goals

Tests should verify behavior through public interfaces, not implementation details.

## Proof Expectations

Commands, tests, screenshots, manual checks, review evidence, or other proof expected later.

## Review Expectations

Reviewer posture, review artifacts, triage expectations, or explicit "no special review" statement.

## Further Notes

Any useful context that does not fit above.
```

## Write Location

Write `PRD.md` into the Codex1 mission artifact tree.
