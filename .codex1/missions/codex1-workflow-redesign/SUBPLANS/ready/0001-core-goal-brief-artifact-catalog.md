---
codex1_template: subplan
template_version: 1
---

# Core Goal Brief Artifact Catalog

<!-- codex1-section: goal -->
## Goal

Replace the current execution-prompt artifact with the current-only goal-brief artifact through CLI-facing behavior.

<!-- codex1-section: slice_type -->
## Slice Type

AFK - the PRD, ADR, and artifact catalog spec settle the user-facing contract.

<!-- codex1-section: linked_prd -->
## Linked PRD

PRD.md

<!-- codex1-section: linked_plan -->
## Linked Plan

PLAN.md

<!-- codex1-section: linked_specs -->
## Linked Specs

- SPECS/0001-artifact-catalog-and-goal-brief-cli-contract.md

<!-- codex1-section: owner -->
## Owner

Main Codex or one worker owning artifact vocabulary, CLI args, template/rendering, inspect, and direct tests.

<!-- codex1-section: current_behavior -->
## Current Behavior

The current artifact model exposes execution-prompt as a generated artifact, writes EXECUTION_PROMPT.md, reports execution_prompt in inspect, and has special rendering for ExecutionPrompt.

<!-- codex1-section: desired_behavior -->
## Desired Behavior

The current artifact model exposes goal-brief as the generated artifact, writes GOAL_BRIEF.md, reports goal_brief mechanically in inspect, and has no current execution-prompt command or generated duplicate.

<!-- codex1-section: key_interfaces -->
## Key Interfaces

- Artifact catalog current vocabulary
- codex1 init artifact descriptors
- codex1 template list/show
- codex1 interview goal-brief
- codex1 inspect inventory
- artifact write event metadata

<!-- codex1-section: scope -->
## Scope

- Rename the current artifact kind from execution-prompt to goal-brief.
- Update singleton path to GOAL_BRIEF.md.
- Update CLI value names and current command behavior.
- Update template title and goal brief sections to describe a native goal brief.
- Update rendering special cases so they apply to goal-brief only.
- Update inspect inventory and human output to report goal_brief mechanically.
- Update direct CLI tests for the new current behavior.

<!-- codex1-section: out_of_scope -->
## Out Of Scope

- Managed setup skill/doc body updates beyond what is required for direct tests.
- README and broad documentation rewrite.
- Native goal creation or set_goal integration.
- Execution-prompt compatibility alias.

<!-- codex1-section: steps -->
## Steps

- Add or deepen the current artifact catalog around artifact id, title, path, singleton/collection shape, and template identity.
- Rename execution-prompt current behavior to goal-brief in CLI-facing code.
- Make goal-brief rendering produce GOAL_BRIEF.md with native goal brief semantics.
- Update focused integration tests for init, template list/show, interview, inspect, and events.
- Run targeted tests for the touched CLI behavior.

<!-- codex1-section: dependencies -->
## Dependencies

- ADRS/0001-current-only-artifact-catalog.md
- SPECS/0001-artifact-catalog-and-goal-brief-cli-contract.md

<!-- codex1-section: blocked_by -->
## Blocked By

- None

<!-- codex1-section: acceptance_criteria -->
## Acceptance Criteria

- codex1 init descriptors include goal-brief and GOAL_BRIEF.md.
- codex1 template list/show exposes goal-brief and Goal Brief.
- codex1 interview goal-brief writes GOAL_BRIEF.md.
- codex1 inspect reports goal_brief mechanically and does not report execution_prompt as current.
- Goal brief output does not instruct Codex to read itself as the first step and does not claim Codex1 owns native goal completion.
- No current execution-prompt command or generated duplicate is introduced.

<!-- codex1-section: expected_proof -->
## Expected Proof

- Targeted cargo test filters for init, template, goal-brief interview, inspect, and event metadata.
- Targeted CLI smoke commands for init, template list/show, interview goal-brief, and inspect.
- PROOFS/ record summarizing changed areas, commands, failures, and accepted risks.

<!-- codex1-section: exit_criteria -->
## Exit Criteria

- The repo compiles and targeted tests for current goal-brief CLI behavior pass.
- Any remaining execution-prompt references are outside this slice or recorded for the managed guidance slice.

<!-- codex1-section: handoff_notes -->
## Handoff Notes

- Do not preserve execution-prompt as a current alias.
- Keep inspect mechanical and anti-oracle.

