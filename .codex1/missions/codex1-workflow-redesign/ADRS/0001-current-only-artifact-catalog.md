---
codex1_template: adr
template_version: 1
---

# Current-Only Artifact Catalog

<!-- codex1-section: status -->
## Status

accepted

<!-- codex1-section: context -->
## Context

The workflow redesign replaces the old execution-prompt concept with the GOAL_BRIEF.md artifact. Keeping code-level legacy aliases would reduce migration pain, but it would also preserve the old vocabulary in the current command surface and make future agents reintroduce stale behavior.

<!-- codex1-section: decision -->
## Decision

Codex1 will use a current-only artifact catalog for generated mission artifacts. The current generated artifact is goal-brief / GOAL_BRIEF.md; execution-prompt / EXECUTION_PROMPT.md may appear only in documentation as legacy reading guidance for old missions, not as a current command alias or generated duplicate.

<!-- codex1-section: options_considered -->
## Options Considered

- Keep execution-prompt as an alias in the CLI for compatibility.
- Generate both EXECUTION_PROMPT.md and GOAL_BRIEF.md during a transition.
- Use a current-only artifact catalog and document old names as legacy reading guidance.

<!-- codex1-section: tradeoffs -->
## Tradeoffs

- Current-only naming creates a cleaner mental model and stronger tests, but old commands will not continue as normal workflow.
- Documentation-level legacy guidance is less automatic than code aliases, but it avoids keeping stale concepts alive in current output.
- The rename touches several modules because artifact vocabulary is currently spread across layout, CLI args, templates, inspect, setup, docs, and tests.

<!-- codex1-section: consequences -->
## Consequences

- The implementation should rename the current enum case, CLI value, singleton path, template title, inspect field, setup guidance, and tests to goal-brief terminology.
- Tests should fail if execution-prompt reappears as current generated CLI or setup output.
- Legacy EXECUTION_PROMPT.md references are acceptable only when clearly marked as older mission reading guidance.

<!-- codex1-section: artifact_links -->
## Links To PRD/Plan/Specs

- PRD.md
- PLAN.md
- SPECS/0001-artifact-catalog-and-goal-brief-cli-contract.md

