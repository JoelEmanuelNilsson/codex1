---
codex1_template: spec
template_version: 1
---

# Artifact Catalog And Goal Brief CLI Contract

<!-- codex1-section: responsibility -->
## Responsibility

Define the current artifact vocabulary and CLI-facing behavior for replacing execution-prompt with goal-brief.

<!-- codex1-section: prd_relevance -->
## PRD Relevance

Satisfies the PRD criteria that the artifact model, CLI template listing, template display, interview writing, inspect inventory, docs, README, and setup bundle use GOAL_BRIEF.md and goal-brief terminology for current artifacts.

<!-- codex1-section: scope -->
## Scope

- Current generated artifact id is goal-brief.
- Current generated singleton file is GOAL_BRIEF.md.
- Current generated template title is Goal Brief.
- The current CLI interview kind is goal-brief.
- The current template list/show surface exposes goal-brief, not execution-prompt.
- The current inspect inventory exposes a mechanical goal_brief count and no execution_prompt current field.
- The artifact catalog remains current-only and does not add execution-prompt aliases.

<!-- codex1-section: non_goals -->
## Non-Goals

- Do not implement native goal creation or set_goal integration.
- Do not support execution-prompt as a current CLI alias unless a later accepted risk explicitly changes this spec.
- Do not make inspect infer readiness, next action, proof sufficiency, or completion.

<!-- codex1-section: expected_behavior -->
## Expected Behavior

- New mission initialization descriptors include goal-brief pointing to GOAL_BRIEF.md.
- New mission initialization does not create GOAL_BRIEF.md until the artifact is written.
- Interviewing goal-brief writes GOAL_BRIEF.md.
- Goal brief markdown clearly frames itself as a native goal brief, not the native goal itself.
- Goal brief copy markers may remain if useful, but their wording must not imply Codex1 owns the native goal.
- Artifact write events record artifact_kind goal-brief for goal brief writes.
- Current human and JSON CLI output use goal-brief terminology.

<!-- codex1-section: interfaces_contracts -->
## Interfaces And Contracts

- Artifact catalog interface: callers can ask for current artifact id, title, singleton path or collection directory, and template identity from one current vocabulary source.
- CLI value contract: user-facing artifact kind names are kebab-case and include goal-brief.
- Filesystem contract: current goal brief artifact path is GOAL_BRIEF.md under the mission root.
- Inspect contract: inventory remains mechanical and reports counts only.

<!-- codex1-section: implementation_notes -->
## Implementation Notes

- Prefer deepening the existing ArtifactKind area rather than scattering another rename across layout, CLI, template, inspect, and command output.
- If clap ValueEnum still requires a separate argument enum, keep the mapping thin and current-only.
- Do not add a compatibility shim unless execution later records a deliberate accepted risk.

<!-- codex1-section: proof_expectations -->
## Proof Expectations

- cargo test covers init descriptors, template list/show, interview goal-brief, inspect goal_brief count, and event artifact_kind.
- Targeted CLI smoke checks exercise init, template list/show goal-brief, interview goal-brief, and inspect.
- Repository search confirms execution-prompt is absent from current CLI-facing code and tests except explicit legacy docs.

<!-- codex1-section: risks -->
## Risks

- The old execution-prompt name appears in many tests and setup strings; broad search is required.
- Renaming serialized JSON fields can break tests that assume execution_prompt inventory.
- Goal brief wording can accidentally claim to complete or create native goals.

<!-- codex1-section: revision_notes -->
## Revision Notes

_Not specified._

