---
codex1_template: proof
template_version: 1
---

# Core Goal Brief Artifact Catalog Proof

<!-- codex1-section: linked_subplan -->
## Linked Subplan

SUBPLANS/done/0001-core-goal-brief-artifact-catalog.md

<!-- codex1-section: linked_spec -->
## Linked Spec

SPECS/0001-artifact-catalog-and-goal-brief-cli-contract.md

<!-- codex1-section: summary_of_changes -->
## Summary Of Changes

Renamed the current execution-prompt artifact contract to goal-brief through CLI-facing behavior. Current init descriptors, template list/show, interview output, inspect inventory, and event metadata now use goal-brief, GOAL_BRIEF.md, and goal_brief. The old execution-prompt command is rejected instead of preserved as a current alias.

<!-- codex1-section: commands_run -->
## Commands Run

- cargo test goal_brief -- --nocapture
- cargo test init_returns_success_envelope -- --nocapture
- cargo test inspect_is_inventory_only -- --nocapture
- cargo test execution_prompt -- --nocapture
- cargo test

<!-- codex1-section: tests_run -->
## Tests Run

- render::tests::renders_goal_brief_suggested_goal_request
- render::tests::goal_brief_requires_completion_and_non_completion_rules
- template_list_and_show_expose_goal_brief
- goal_brief_interview_writes_native_goal_brief
- removed_execution_prompt_command_fails_through_argument_parser
- init_returns_success_envelope
- inspect_is_inventory_only

<!-- codex1-section: manual_checks -->
## Manual Checks

- Searched src and tests for stale current ExecutionPrompt, execution-prompt, EXECUTION_PROMPT, and execution_prompt references
- Smoked init, template list/show goal-brief, interview goal-brief, and inspect in a temporary repo

<!-- codex1-section: changed_areas -->
## Changed Areas

- ArtifactKind and singleton path model
- CLI artifact argument mapping
- Template registry and goal brief sections
- Markdown rendering
- Inspect inventory and human output
- Event artifact_kind metadata through artifact serialization
- CLI integration tests

<!-- codex1-section: failures -->
## Failures

- Initial red run failed because ArtifactKind::GoalBrief did not exist yet, as expected
- A combined cargo test command with two filters was invalid and rerun with valid filters

<!-- codex1-section: accepted_risks -->
## Accepted Risks

- Legacy execution-prompt strings remain only in removal/negative-test contexts and are covered by search proof

<!-- codex1-section: evidence_links -->
## Evidence Links

- src/layout.rs
- src/template.rs
- src/render.rs
- src/inspect.rs
- src/cli.rs
- tests/cli.rs
