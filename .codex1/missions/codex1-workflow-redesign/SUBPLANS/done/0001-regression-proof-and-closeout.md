---
codex1_template: subplan
template_version: 1
---

# Regression Proof And Closeout

<!-- codex1-section: goal -->
## Goal

Prove the workflow redesign end to end, record evidence artifacts, triage any review findings, and close the mission without claiming native goal completion.

<!-- codex1-section: slice_type -->
## Slice Type

AFK - proof and closeout expectations are defined by the PRD and verification spec.

<!-- codex1-section: linked_prd -->
## Linked PRD

PRD.md

<!-- codex1-section: linked_plan -->
## Linked Plan

PLAN.md

<!-- codex1-section: linked_specs -->
## Linked Specs

- SPECS/0001-legacy-terminology-and-anti-oracle-verification-contract.md

<!-- codex1-section: owner -->
## Owner

Main Codex owning final verification, review/triage coordination, proof artifacts, and closeout.

<!-- codex1-section: current_behavior -->
## Current Behavior

After implementation slices, the repo may still contain stale current-use terminology or unrecorded evidence.

<!-- codex1-section: desired_behavior -->
## Desired Behavior

The repo has passing tests, clear search evidence, no stale current execution-prompt workflow, recorded proofs, triaged review findings when applicable, and closeout that audits PRD satisfaction.

<!-- codex1-section: key_interfaces -->
## Key Interfaces

- cargo fmt
- cargo test
- cargo clippy -- -D warnings
- codex1 CLI smoke checks
- Repository terminology search
- PROOFS/
- REVIEWS/
- TRIAGE/
- CLOSEOUT.md

<!-- codex1-section: scope -->
## Scope

- Run full formatting, test, clippy, smoke, and terminology-search proof.
- Record proof artifacts for completed implementation slices if not already recorded.
- Run or request review where useful and triage findings.
- Write CLOSEOUT.md only after auditing PRD success criteria against evidence.
- Record accepted risks or deferred work if completion cannot be fully proven.

<!-- codex1-section: out_of_scope -->
## Out Of Scope

- New product behavior beyond PRD scope.
- Browser-native dogfood.
- Native goal completion from Codex1 artifacts.
- Issue tracker publication unless the user separately asks.

<!-- codex1-section: steps -->
## Steps

- Run the required proof commands and collect outputs.
- Search for stale current terminology and classify any remaining legacy mentions.
- Record proof artifacts with commands, changed areas, failures, and accepted risks.
- Review and triage any findings.
- Write closeout after evidence audit.

<!-- codex1-section: dependencies -->
## Dependencies

- SUBPLANS/ready/0001-core-goal-brief-artifact-catalog.md
- SUBPLANS/ready/0001-managed-workflow-guidance-goal-brief.md

<!-- codex1-section: blocked_by -->
## Blocked By

- None

<!-- codex1-section: acceptance_criteria -->
## Acceptance Criteria

- cargo fmt succeeds.
- cargo test succeeds.
- cargo clippy -- -D warnings succeeds or any failure is recorded as an accepted risk with reason.
- Targeted CLI smoke checks for goal-brief and setup succeed.
- Search proof shows EXECUTION_PROMPT.md and execution-prompt are absent from current workflow or explicitly legacy-only.
- PROOFS/ contains evidence for completed slices.
- CLOSEOUT.md audits all PRD success criteria and remaining risks.

<!-- codex1-section: expected_proof -->
## Expected Proof

- Command outputs summarized in PROOFS/.
- Search results summarized in PROOFS/.
- Review/triage records if review is performed.
- CLOSEOUT.md with completed, superseded, paused, deferred, and risky work.

<!-- codex1-section: exit_criteria -->
## Exit Criteria

- The PRD is satisfied or all non-satisfaction is explicitly deferred with reason.
- No required proof remains missing.
- Closeout is written and does not claim to complete native /goal by itself.

<!-- codex1-section: handoff_notes -->
## Handoff Notes

- Do not use codex1 inspect, events, receipts, or folder placement as proof of completion.
- Use Codex judgment over the evidence to decide native goal completion.
