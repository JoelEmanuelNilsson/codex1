---
codex1_template: subplan
template_version: 1
---

# Managed Workflow Guidance Goal Brief

<!-- codex1-section: goal -->
## Goal

Update setup-managed skills, supporting docs, README, and workflow docs to teach the redesigned local-first goal-brief workflow.

<!-- codex1-section: slice_type -->
## Slice Type

AFK - the PRD and managed workflow guidance spec settle the required concepts and non-goals.

<!-- codex1-section: linked_prd -->
## Linked PRD

PRD.md

<!-- codex1-section: linked_plan -->
## Linked Plan

PLAN.md

<!-- codex1-section: linked_specs -->
## Linked Specs

- SPECS/0001-managed-workflow-guidance-contract.md
- SPECS/0001-legacy-terminology-and-anti-oracle-verification-contract.md

<!-- codex1-section: owner -->
## Owner

Main Codex or one worker owning setup-managed bodies, repo docs, README, and setup tests.

<!-- codex1-section: current_behavior -->
## Current Behavior

Setup-managed skills and docs still teach EXECUTION_PROMPT.md as the current planning output and describe a pasteable execution prompt rather than a native goal brief.

<!-- codex1-section: desired_behavior -->
## Desired Behavior

Setup-managed skills and docs teach local-first missions, grill-with-docs clarify, synthesis PRDs, agentic E2E plan packs, GOAL_BRIEF.md, evidence artifacts, native goal ownership, architecture lens/refactor modes, and honest execution discipline.

<!-- codex1-section: key_interfaces -->
## Key Interfaces

- codex1 setup install/status/doctor
- Managed skills: codex1, clarify, create-prd, plan
- Managed supporting docs: workflow, domain, artifact briefs, goal brief format
- README and docs/skill-workflows.md
- AGENTS.md managed guidance block

<!-- codex1-section: scope -->
## Scope

- Update setup-managed overview, clarify, create-prd, and plan skill bodies.
- Rename or replace the execution prompt format support doc with goal brief format support.
- Update managed file lists, marker version, and legacy bundle handling if the managed support doc path changes.
- Update docs/agents, docs/artifact-model.md, docs/cli-contract.md, docs/skill-workflows.md, README, and AGENTS managed guidance language.
- Update setup materialization tests to assert goal-brief workflow concepts.
- Clearly mark any remaining EXECUTION_PROMPT.md mention as legacy reading guidance only.

<!-- codex1-section: out_of_scope -->
## Out Of Scope

- Core CLI artifact rename already covered by the prior slice.
- Browser-native dogfood.
- Issue tracker publishing.
- Native goal RPC or set_goal implementation.

<!-- codex1-section: steps -->
## Steps

- Update setup-managed content constants and current checked-in managed files together.
- Update docs and README to match the same vocabulary.
- Update setup tests for generated skills/docs and marker file lists.
- Run setup install/status tests and inspect generated content for stale current terminology.

<!-- codex1-section: dependencies -->
## Dependencies

- SUBPLANS/ready/0001-core-goal-brief-artifact-catalog.md should complete first so docs can name the real current CLI behavior.

<!-- codex1-section: blocked_by -->
## Blocked By

- None

<!-- codex1-section: acceptance_criteria -->
## Acceptance Criteria

- Fresh setup install materializes current goal-brief guidance.
- Managed plan guidance writes GOAL_BRIEF.md as a native goal brief and does not teach EXECUTION_PROMPT.md as current.
- Managed clarify guidance follows grill-with-docs and does not create a standalone clarification artifact.
- Managed create-prd guidance remains local-first and non-publishing.
- Docs explain proof/review/triage/closeout as evidence artifacts only.
- Docs explain architecture lens and architecture refactor mission modes.
- Setup status reports the new bundle current in a fresh temp repo.

<!-- codex1-section: expected_proof -->
## Expected Proof

- cargo test coverage for setup install/status/doctor and managed content checks.
- Targeted setup install smoke in a temp repo.
- Search results for stale current EXECUTION_PROMPT.md or execution-prompt wording.
- PROOFS/ record for the slice.

<!-- codex1-section: exit_criteria -->
## Exit Criteria

- Managed setup output, checked-in managed files, and docs agree on the redesigned workflow.
- Any legacy mention is clearly marked and recorded for final verification.

<!-- codex1-section: handoff_notes -->
## Handoff Notes

- Keep setup mechanical; do not add semantic readiness or native goal state.
- If marker version changes, update setup backup/legacy behavior intentionally.

