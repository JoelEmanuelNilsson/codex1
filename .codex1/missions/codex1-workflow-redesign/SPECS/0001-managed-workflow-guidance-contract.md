---
codex1_template: spec
template_version: 1
---

# Managed Workflow Guidance Contract

<!-- codex1-section: responsibility -->
## Responsibility

Define how setup-managed skills and docs should teach the redesigned local-first Codex1 workflow.

<!-- codex1-section: prd_relevance -->
## PRD Relevance

Satisfies the PRD criteria for managed skills, setup bundle output, repo docs, README, and workflow notes.

<!-- codex1-section: scope -->
## Scope

- Overview guidance presents Codex1 as local-first mission artifacts plus native goal briefs.
- Clarify guidance follows grill-with-docs: one meaningful question at a time, inspect before asking, update CONTEXT.md inline, and write ADRs only when earned.
- Create PRD guidance synthesizes known context and does not publish to issue trackers.
- Plan guidance produces an agentic E2E plan pack with specs phase, ready subplans, proof expectations, and GOAL_BRIEF.md.
- Artifact briefs explain proof/review/triage/closeout as evidence artifacts.
- Workflow docs explain native /goal owns continuation, accounting, and completion.
- Setup marker version and managed file lists are updated if managed file names change.

<!-- codex1-section: non_goals -->
## Non-Goals

- Do not implement Browser-native dogfood.
- Do not add issue tracker publishing.
- Do not add a standalone clarification artifact.
- Do not create or inspect native goal state from setup guidance.

<!-- codex1-section: expected_behavior -->
## Expected Behavior

- Fresh setup install materializes skills and supporting docs with goal-brief terminology.
- Fresh setup install does not materialize EXECUTION-PROMPT-FORMAT.md as a current managed file if the format file is renamed.
- Setup status reports current for the new managed bundle.
- Setup disable/uninstall removes only managed files and does not remove mission artifacts or native goal state.
- Generated guidance clearly says Codex1 evidence artifacts support Codex judgment but do not own completion.

<!-- codex1-section: interfaces_contracts -->
## Interfaces And Contracts

- Managed bundle contract: setup install/status/doctor use deterministic expected bodies and managed file lists.
- Skill contract: $clarify, $create-prd, and $plan remain the user-facing workflow names.
- Goal brief format contract: the supporting format document is named for goal briefs and describes native goal brief contents.
- Legacy wording contract: EXECUTION_PROMPT.md may appear only in clearly marked legacy reading guidance.

<!-- codex1-section: implementation_notes -->
## Implementation Notes

- The setup module currently embeds long managed bodies; this mission may edit it directly, but should avoid making setup mechanics harder to understand.
- If a managed support doc is renamed, update marker versions and legacy bundle file lists deliberately.
- Tests should assert important concepts rather than exact full prose.

<!-- codex1-section: proof_expectations -->
## Proof Expectations

- cargo test covers setup install materialization and setup status.
- Targeted setup install smoke test in a temporary repo verifies generated files and goal-brief wording.
- Repository search verifies current managed outputs do not teach EXECUTION_PROMPT.md as the current artifact.

<!-- codex1-section: risks -->
## Risks

- Setup expected body changes can make existing local managed files appear stale until setup is re-run.
- Renaming a managed support doc requires careful disable/uninstall behavior for previous marker versions.
- Docs can use correct artifact names while still implying Codex1 owns goal completion; review must check this.

<!-- codex1-section: revision_notes -->
## Revision Notes

_Not specified._

