---
codex1_template: proof
template_version: 1
---

# Managed Workflow Guidance Goal Brief Proof

<!-- codex1-section: linked_subplan -->
## Linked Subplan

SUBPLANS/done/0001-managed-workflow-guidance-goal-brief.md

<!-- codex1-section: linked_spec -->
## Linked Spec

SPECS/0001-managed-workflow-guidance-contract.md

<!-- codex1-section: summary_of_changes -->
## Summary Of Changes

Updated managed setup skill bodies, repo docs, README, AGENTS guidance, and setup bundle metadata to teach the local-first goal-brief workflow. Fresh setup output now materializes GOAL-BRIEF-FORMAT.md and no current EXECUTION-PROMPT-FORMAT.md. Setup marker version is bumped and old managed execution prompt format files are treated as legacy removable setup files.

<!-- codex1-section: commands_run -->
## Commands Run

- cargo test setup_ -- --nocapture
- cargo run -- --json setup status
- cargo test
- cargo fmt
- cargo clippy -- -D warnings

<!-- codex1-section: tests_run -->
## Tests Run

- setup_install_materializes_repo_scoped_guidance_without_hooks
- setup_status_reports_bundle_state_only
- setup_enable_repairs_stale_managed_skill_and_marker
- setup::tests::marker_body_matches_expected_files
- checked_in_docs_mark_execution_prompt_mentions_as_legacy_only

<!-- codex1-section: manual_checks -->
## Manual Checks

- Verified root setup status reports marker, skills, supporting docs, and AGENTS guidance as current
- Searched README, AGENTS, docs, and .agents for stale current execution prompt terminology
- Smoked setup install and setup status in a temporary repo

<!-- codex1-section: changed_areas -->
## Changed Areas

- src/setup.rs managed bundle constants and bodies
- .agents managed skills and plan support docs
- docs/agents workflow and artifact briefs
- README, AGENTS, CLI contract, artifact model, skill workflow notes
- .codex1/setup-bundle.json

<!-- codex1-section: failures -->
## Failures

- cargo clippy initially reported a manual_contains warning in setup aggregation; fixed by using contains

<!-- codex1-section: accepted_risks -->
## Accepted Risks

- src/setup.rs keeps the exact legacy execution prompt support-doc body so old managed bundles can be removed safely

<!-- codex1-section: evidence_links -->
## Evidence Links

- src/setup.rs
- .agents/skills/plan/GOAL-BRIEF-FORMAT.md
- docs/agents/codex1-workflow.md
- docs/agents/codex1-artifact-briefs.md
- README.md
- tests/cli.rs
