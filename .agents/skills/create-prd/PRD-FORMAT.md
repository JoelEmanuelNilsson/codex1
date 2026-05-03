# PRD Format

Use this when `$create-prd` writes `PRD.md`. This is adapted from the reference `to-prd` format, but Codex1 keeps the PRD local and does not publish to an issue tracker.

## Required Quality Bar

The PRD must be sufficient for `$plan` to design execution without reconstructing product intent. Write from the user's perspective first, then capture implementation and testing decisions.

## Template

```md
## Problem Statement

The problem the user is facing, from the user's perspective.

## Solution

The solution, from the user's perspective.

## User Stories

A long numbered list of user stories:

1. As an <actor>, I want <feature>, so that <benefit>.

Cover all major behavior, actors, edge cases, and artifact interactions.

## Success Criteria

Observable facts that make the PRD satisfied.

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

## Out Of Scope

What this PRD intentionally does not include.

## Proof Expectations

Commands, tests, screenshots, manual checks, review evidence, or other proof expected later.

## Review Expectations

Reviewer posture, review artifacts, triage expectations, or explicit "no special review" statement.

## Further Notes

Any useful context that does not fit above.
```

## Local-only Rule

Do not publish this PRD to GitHub Issues, Linear, Jira, GitLab, or another tracker. Write it into the Codex1 mission artifact tree.
