---
codex1_template: proof
template_version: 1
---

# Regression Proof And Closeout Proof

<!-- codex1-section: linked_subplan -->
## Linked Subplan

SUBPLANS/done/0001-regression-proof-and-closeout.md

<!-- codex1-section: linked_spec -->
## Linked Spec

SPECS/0001-legacy-terminology-and-anti-oracle-verification-contract.md

<!-- codex1-section: summary_of_changes -->
## Summary Of Changes

Ran final regression proof for the Codex1 workflow redesign. Verified formatting, full tests, clippy, setup status, mission inspect, smoke checks, and terminology searches. Recorded that remaining execution-prompt terminology is confined to legacy-removal code, explicit legacy docs, CONTEXT vocabulary, mission planning/proof context, and negative tests proving the old command surface is gone.

<!-- codex1-section: commands_run -->
## Commands Run

- cargo fmt
- cargo test
- cargo clippy -- -D warnings
- cargo run -- --json setup status
- cargo run -- --json --mission codex1-workflow-redesign inspect
- targeted temp-repo smoke for init, template list, template show goal-brief, interview goal-brief, inspect, setup install, and setup status
- rg stale execution-prompt terminology excluding target, git, and setup backups
- rg goal-brief terminology across src, tests, docs, managed skills, and mission artifacts

<!-- codex1-section: tests_run -->
## Tests Run

- 8 unit tests passed
- 62 CLI integration tests passed
- checked_in_docs_mark_execution_prompt_mentions_as_legacy_only
- removed_execution_prompt_command_fails_through_argument_parser
- setup_install_materializes_repo_scoped_guidance_without_hooks
- goal_brief_interview_writes_native_goal_brief
- template_list_and_show_expose_goal_brief

<!-- codex1-section: manual_checks -->
## Manual Checks

- Classified remaining EXECUTION_PROMPT and execution-prompt search hits
- Confirmed root setup status reports marker, skills, supporting docs, and guidance as current
- Confirmed mission inspect reports goal_brief: 1, proofs: 2 before final proof, and no mechanical warnings
- Confirmed temporary repo smoke writes GOAL_BRIEF.md and does not write EXECUTION_PROMPT.md

<!-- codex1-section: changed_areas -->
## Changed Areas

- Regression tests
- Proof artifacts
- Subplan lifecycle movement
- Closeout evidence

<!-- codex1-section: failures -->
## Failures

- None in final proof run

<!-- codex1-section: accepted_risks -->
## Accepted Risks

- Search results still include legacy-removal code in src/setup.rs and explicit legacy/context/test references; these are intentional and not current workflow guidance
- No separate REVIEW/TRIAGE artifact was created because tests, smoke, and terminology proof found no actionable review findings requiring adjudication

<!-- codex1-section: evidence_links -->
## Evidence Links

- PROOFS/0001-core-goal-brief-artifact-catalog-proof.md
- PROOFS/0001-managed-workflow-guidance-goal-brief-proof.md
- tests/cli.rs
- README.md
- docs/cli-contract.md
- docs/artifact-model.md
- src/setup.rs
