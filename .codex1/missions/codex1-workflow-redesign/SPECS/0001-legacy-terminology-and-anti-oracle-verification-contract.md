---
codex1_template: spec
template_version: 1
---

# Legacy Terminology And Anti-Oracle Verification Contract

<!-- codex1-section: responsibility -->
## Responsibility

Define how the mission proves stale execution-prompt terminology is removed from current workflow while preserving anti-oracle boundaries.

<!-- codex1-section: prd_relevance -->
## PRD Relevance

Satisfies the PRD criteria for terminology regression tests, legacy guidance boundaries, evidence-only artifacts, and native goal ownership.

<!-- codex1-section: scope -->
## Scope

- Tests or scripted checks distinguish current workflow references from explicitly marked legacy references.
- Current CLI, setup-generated skills/docs, README, and agent docs do not present execution-prompt as current.
- Docs may mention EXECUTION_PROMPT.md only under legacy reading guidance for older missions.
- Proof/review/triage/closeout remain evidence artifacts and are not described as completion authority.
- Inspect, setup status, events, and receipts remain mechanical helpers and not readiness or completion proof.
- Final mission evidence records commands, tests, search results, failures, accepted risks, and closeout.

<!-- codex1-section: non_goals -->
## Non-Goals

- Do not enforce natural-language docs with brittle exact-prose tests beyond managed setup output.
- Do not make event logs, receipts, or inspect determine semantic state.
- Do not require Browser-based dogfood for this CLI/docs mission.

<!-- codex1-section: expected_behavior -->
## Expected Behavior

- Full tests pass after the rename and guidance updates.
- Clippy passes with warnings denied where practical.
- Search proof lists any remaining execution-prompt or EXECUTION_PROMPT references and classifies them as legacy-only or requiring fix.
- Closeout audits each PRD success criterion against proofs and review/triage records.

<!-- codex1-section: interfaces_contracts -->
## Interfaces And Contracts

- Proof artifact contract: record commands run, tests run, changed areas, failures, accepted risks, and evidence links.
- Review artifact contract: reviewer opinions are input to Codex triage and do not mutate mission truth.
- Closeout contract: summarize PRD satisfaction and remaining risks without completing native /goal itself.

<!-- codex1-section: implementation_notes -->
## Implementation Notes

- Prefer tests that exercise public CLI behavior and managed setup output.
- Use repository search as proof, but do not build a complex semantic doc parser unless stale terminology keeps recurring.
- If a legacy mention remains, make its legacy status visually obvious in the surrounding prose.

<!-- codex1-section: proof_expectations -->
## Proof Expectations

- cargo fmt
- cargo test
- cargo clippy -- -D warnings where practical
- Targeted CLI smoke checks for goal-brief and setup
- Search for EXECUTION_PROMPT, execution-prompt, Execution Prompt, GOAL_BRIEF, goal-brief, and Goal Brief
- PROOFS/ records for completed slices
- CLOSEOUT.md after evidence audit

<!-- codex1-section: risks -->
## Risks

- A search-only proof can miss wording that implies old semantics without using old exact terms.
- A full prose lock test can become brittle and slow future documentation improvements.
- Closeout language can accidentally sound like it completes the native goal.

<!-- codex1-section: revision_notes -->
## Revision Notes

_Not specified._

