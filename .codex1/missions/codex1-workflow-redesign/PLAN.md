---
codex1_template: plan
template_version: 1
---

# Codex1 Workflow Redesign Plan

<!-- codex1-section: mission_link -->
## Mission Link

.codex1/missions/codex1-workflow-redesign/PRD.md

<!-- codex1-section: strategy_thesis -->
## Strategy Thesis

Implement the workflow redesign as a current-artifact vocabulary change, not as a second goal engine. Start by making the artifact catalog current-only around GOAL_BRIEF.md, then update managed skills/docs/setup output to teach the clarified local-first workflow, then prove the rename and anti-oracle boundaries end to end.

<!-- codex1-section: workstreams -->
## Workstreams

- Artifact catalog and CLI contract: replace the current execution-prompt artifact with goal-brief terminology and GOAL_BRIEF.md while preserving mechanical artifact behavior.
- Goal brief template and rendering: make the generated artifact a native goal brief that helps Codex create/refine a mission goal instead of acting as a sacred final prompt.
- Managed workflow guidance: update setup-managed skills, supporting docs, README, and workflow notes to reflect clarify, create-prd, plan pack, goal brief, evidence, and closeout.
- Evidence and anti-oracle posture: preserve proof/review/triage/closeout as evidence artifacts and keep inspect/setup/events/receipts mechanical.
- Regression proof: add CLI and docs tests that catch stale current-use execution-prompt terminology while allowing explicit legacy reading guidance.

<!-- codex1-section: phases -->
## Phases

- Phase 1: Record the current-only artifact catalog ADR and implement the core goal-brief artifact contract through CLI-facing behavior.
- Phase 2: Update managed skills, setup bundle content, and repo docs so the generated guidance matches the settled workflow.
- Phase 3: Add terminology and anti-oracle regression coverage, run full proof, and close the mission evidence.

<!-- codex1-section: research_posture -->
## Research Posture

No external research is needed. The uncertainty is local architecture and workflow consistency, already resolved through clarification and code inspection. Treat any additional discovery during implementation as repo inspection, not external research.

<!-- codex1-section: risk_map -->
## Risk Map

- The old execution-prompt terminology may survive in setup-generated strings or tests even if the core CLI is renamed.
- Adding legacy aliases in code would quietly preserve the old concept and weaken the current-only artifact catalog decision.
- Goal brief wording may drift into claiming Codex1 owns native goal creation or completion.
- Inspect inventory or docs might accidentally imply proof sufficiency, readiness, or completion.
- A broad rename without focused tests could pass while leaving stale human-facing wording.

<!-- codex1-section: artifact_index -->
## Artifact Index

- PRD.md is the outcome contract.
- ADRS/0001-current-only-artifact-catalog.md records the current-only artifact catalog decision.
- SPECS/0001-artifact-catalog-and-goal-brief-cli-contract.md defines the CLI-facing artifact contract.
- SPECS/0001-managed-workflow-guidance-contract.md defines setup-managed docs and skill output.
- SPECS/0001-legacy-terminology-and-anti-oracle-verification-contract.md defines regression proof expectations.
- SUBPLANS/ready/0001-core-goal-brief-artifact-catalog.md is the first executable slice.
- SUBPLANS/ready/0001-managed-workflow-guidance-goal-brief.md is the second executable slice.
- SUBPLANS/ready/0001-regression-proof-and-closeout.md is the final verification slice.
- GOAL_BRIEF.md is the native goal brief for executing the whole mission.

<!-- codex1-section: review_posture -->
## Review Posture

Review should focus on current-vs-legacy terminology, accidental native-goal ownership, anti-oracle regressions, setup bundle drift, and whether tests prove the workflow through public CLI behavior.

<!-- codex1-section: recommended_next_slices -->
## Recommended Next Slices

- Run SUBPLANS/ready/0001-core-goal-brief-artifact-catalog.md first.
- Run SUBPLANS/ready/0001-managed-workflow-guidance-goal-brief.md second.
- Run SUBPLANS/ready/0001-regression-proof-and-closeout.md last.
