---
codex1_template: prd
template_version: 1
---

# Codex1 Workflow Redesign Around Native Goals

<!-- codex1-section: original_request -->
## Original Request

Redesign the Codex1 workflow so it fits the user's current solo Codex workflow: clarify with grill-with-docs discipline, synthesize a local PRD, create an agentic E2E plan pack, use native /goal as the execution center, and update the repo so Codex1 feels native rather than bolted on.

<!-- codex1-section: problem_statement -->
## Problem Statement

The user has a working but imperfect solo workflow built around grill-with-docs, PRD synthesis, diagnose, occasional TDD, architecture review, worktrees, and native /goal. Codex1 currently has the right high-level boundary, but its repo guidance and CLI still frame planning around EXECUTION_PROMPT.md and do not fully express the settled local-first workflow. This creates translation friction: agents may treat Codex1 as a second workflow engine, may publish PRDs to issue trackers, may over-ask in PRD creation, may skip architecture thinking until too late, or may treat a generated prompt as more authoritative than native /goal.

<!-- codex1-section: solution -->
## Solution

Make Codex1 a local-first mission artifact workflow that prepares Codex to set and execute a native mission goal. The flow should be $clarify, $create-prd, $plan, native goal creation, execution, evidence, and closeout. $clarify follows grill-with-docs by updating CONTEXT.md and ADRs inline rather than creating a standalone clarification artifact. $create-prd synthesizes known context into a local PRD without publishing to an issue tracker. $plan produces an agentic E2E plan pack with a specs phase, ready subplans, acceptance criteria, proof expectations, and a GOAL_BRIEF.md artifact that helps Codex create or refine the actual native goal. Native /goal owns continuation and completion. Codex1 owns durable artifacts and evidence only.

<!-- codex1-section: interpreted_destination -->
## Interpreted Destination

A repo-local Codex1 workflow where the docs, managed skills, artifact vocabulary, CLI templates, inspect output, setup bundle, tests, and README all consistently express local-first missions, native goal briefs, agentic E2E plan packs, evidence-only proof/review/closeout, and the replacement of EXECUTION_PROMPT.md with GOAL_BRIEF.md.

<!-- codex1-section: user_stories -->
## User Stories

1. As a solo Codex1 user, I want Codex1 to be local-first, so that my mission context lives in the repo without forcing issue-tracker ceremony.
2. As a solo Codex1 user, I want GitHub Issues and other trackers to be explicit opt-in publishing destinations, so that private solo work stays lightweight.
3. As a Codex1 user, I want $clarify to behave like grill-with-docs, so that Codex challenges my idea against docs, code, domain language, and ADR-worthy tradeoffs.
4. As a Codex1 user, I want $clarify to update CONTEXT.md inline when terms crystallize, so that project language becomes durable.
5. As a Codex1 user, I want $clarify to write ADRs only for durable, surprising tradeoffs, so that decisions are recorded without ceremony.
6. As a Codex1 user, I do not want a standalone clarification artifact, so that the clarify phase follows the existing grill-with-docs model.
7. As a Codex1 user, I want $create-prd to synthesize rather than re-interview me, so that PRD creation feels calm and deterministic after clarification.
8. As a Codex1 user, I want $create-prd to record assumptions and open risks instead of blocking on minor missing details, so that planning can proceed.
9. As a Codex1 user, I want $create-prd to ask only when trusted sources contradict or a missing answer would make the PRD misleading, so that only meaningful uncertainty interrupts me.
10. As a Codex1 user, I want $plan to produce an agentic E2E plan pack, so that a mission can be executed end to end by Codex.
11. As a Codex1 user, I want $plan to ask questions only when answers change executable shape, so that planning stays focused.
12. As a Codex1 user, I want every plan to include a specs phase, so that contract thinking happens deliberately.
13. As a Codex1 user, I want spec files only when they earn their keep, so that tiny or docs-only missions avoid fake paperwork.
14. As a Codex1 user, I want ready subplans to be vertical, testable, and independently provable, so that /goal can execute the whole mission safely.
15. As a Codex1 user, I want unresolved human decisions labeled HITL and kept out of ready execution, so that agents do not guess past important ambiguity.
16. As a Codex1 user, I want the normal native goal to complete the whole Codex1 mission plan, so that I do not have to create one goal per slice.
17. As a Codex1 user, I want Codex1 to prepare a native goal brief rather than claim to own the goal, so that Codex can create the best native goal from the plan pack.
18. As a Codex1 user, I want GOAL_BRIEF.md to replace EXECUTION_PROMPT.md, so that artifact names match the settled mental model.
19. As a Codex1 user, I want old EXECUTION_PROMPT.md files treated only as legacy reading guidance, so that old missions remain understandable without preserving the old concept in current generation.
20. As a Codex1 user, I want Codex1 proof, review, triage, and closeout artifacts to be evidence only, so that native /goal remains the completion authority.
21. As a Codex1 user, I want proof artifacts to record commands, tests, manual checks, failures, and accepted risks, so that mission completion has durable support.
22. As a Codex1 user, I want review artifacts to record opinions and findings, so that review can inform execution without becoming an automatic gate.
23. As a Codex1 user, I want triage artifacts to record Codex's adjudication of reviews, so that accepted, rejected, stale, duplicate, and deferred findings are clear.
24. As a Codex1 user, I want closeout artifacts to audit PRD satisfaction against evidence, so that goal completion has a reliable final summary.
25. As a Codex1 user, I want architecture review to work as both a planning lens and a refactor mission type, so that design thinking can happen before implementation or as its own mission.
26. As a Codex1 user, I want execution to default to TDD where it fits, so that behavior changes get external-behavior tests before implementation.
27. As a Codex1 user, I want diagnose-first workflow for hard bugs, so that bug fixes start with a trustworthy feedback loop.
28. As a Codex1 user, I want non-code and docs-only changes to use appropriate proof rather than fake TDD, so that verification remains honest.
29. As a maintainer, I want the artifact kind model, templates, docs, and setup bundle to use goal-brief terminology, so that generated guidance is consistent.
30. As a maintainer, I want tests to reject stale execution-prompt terminology in current CLI output and docs, so that the old model does not creep back.
31. As a future Codex session, I want the workflow docs to explain that native /goal owns persistence, continuation, accounting, and completion, so that Codex1 artifacts are not mistaken for goal state.
32. As a future Codex session, I want the PRD and plan artifacts to be sufficient for /goal execution, so that it can continue work without reconstructing the conversation.
33. As a future Codex session, I want CONTEXT.md vocabulary to guide implementation, so that terms like mission goal, goal brief, and evidence artifact stay consistent.
34. As a future Codex session, I want Browser-native dogfood excluded from this redesign, so that the mission stays focused.

<!-- codex1-section: success_criteria -->
## Success Criteria

- The current Codex1 workflow docs describe the flow as $clarify, $create-prd, $plan, native goal creation from a goal brief, execution, evidence, and closeout.
- The managed skills generated by setup describe $clarify as grill-with-docs-style discovery that updates CONTEXT.md and ADRs inline without creating a standalone clarification artifact.
- The managed create-prd skill describes local PRD synthesis and does not publish to an issue tracker.
- The managed plan skill describes an agentic E2E plan pack, the specs phase, ready subplans, AFK/HITL labeling, proof expectations, and GOAL_BRIEF.md.
- The artifact model uses GOAL_BRIEF.md and goal-brief terminology for current artifacts.
- The CLI artifact kind currently exposed as execution-prompt is replaced by goal-brief terminology and writes GOAL_BRIEF.md.
- Current CLI template listing, template display, interview writing, inspect inventory, docs, README, and setup bundle output no longer present EXECUTION_PROMPT.md as the current artifact.
- Docs mention EXECUTION_PROMPT.md only as legacy reading guidance for older missions, not as a current generated artifact.
- Proof, review, triage, and closeout are described as evidence artifacts that support Codex judgment but do not own completion.
- Native /goal remains documented as the owner of persistence, continuation, accounting, and completion.
- Architecture review is documented as both a planning lens and a standalone refactor mission type.
- Execution discipline is documented as TDD where it fits, diagnose-first for hard bugs, and appropriate proof for non-code changes.
- Tests cover the goal-brief rename, current artifact paths, current command/template output, managed setup materialization, docs regressions, and anti-oracle boundaries.
- No issue tracker publication is added as part of this workflow.
- Browser-native dogfood is not included in this mission beyond being explicitly out of scope.

<!-- codex1-section: module_sketch -->
## Module Sketch

- Artifact kind model: rename the current execution-prompt artifact concept to goal-brief while preserving the rest of the artifact tree and anti-oracle boundary.
- Mission layout: write GOAL_BRIEF.md for the goal brief singleton and keep older EXECUTION_PROMPT.md only as legacy documentation, not current output.
- Template registry and renderer: expose a goal-brief template whose sections describe native goal brief content rather than a sacred pasteable execution prompt.
- CLI command surface: update template and interview commands so current user-facing names and JSON envelopes use goal-brief terminology.
- Inspect inventory: report goal-brief inventory mechanically without inferring goal readiness, completion, or next action.
- Setup bundle: regenerate managed skill bodies and docs so installed repos receive the redesigned workflow language.
- Managed skills: update codex1, clarify, create-prd, and plan guidance to match local-first missions, grill-with-docs clarify, synthesis PRDs, agentic E2E plan packs, and goal briefs.
- Documentation: update README, CLI contract, artifact model, skill workflow notes, and agent docs to use the settled vocabulary.
- Tests: update integration and regression coverage around artifact descriptors, singleton paths, template output, setup materialization, docs searches, and legacy terminology boundaries.
- Deep-module opportunity: keep artifact vocabulary centralized so goal-brief naming is not duplicated across layout, templates, inspect, setup, and docs.

<!-- codex1-section: non_goals -->
## Non-Goals

- Do not implement native goals inside Codex1.
- Do not call native goal RPCs from Codex1.
- Do not create, inspect, mirror, or complete native goals from Codex1.
- Do not publish PRDs or plans to GitHub Issues, Linear, Jira, GitLab, or any other issue tracker.
- Do not add a standalone clarification artifact.
- Do not make proof, review, triage, closeout, events, receipts, setup status, or inspect into readiness or completion authority.
- Do not implement Browser-native dogfood in this mission.
- Do not preserve EXECUTION_PROMPT.md as a current duplicate generated artifact.
- Do not add compatibility shims that keep the old execution-prompt command as a normal current workflow unless planning later finds a hard compatibility requirement.
- Do not redesign worktree creation in this mission.

<!-- codex1-section: constraints -->
## Constraints

- Codex1 stays local-first; issue trackers are explicit opt-in only.
- Native /goal owns the active objective, continuation, pause/resume, accounting, and completion.
- Codex remains the semantic judge; Codex1 commands stay mechanical.
- Mission artifacts must stay durable and human-readable.
- Ready subplans must be executable without unresolved human decisions.
- Specs are a required planning checkpoint, but dedicated spec files are conditional.
- Execution proof must fit the work; TDD is expected for behavior/code changes but not for docs-only or mechanical artifact edits.
- Existing anti-oracle guarantees must be preserved.
- Managed setup output must stay deterministic and testable.
- Current repo guidance should use CONTEXT.md vocabulary.

<!-- codex1-section: verified_context -->
## Verified Context

- CONTEXT.md now defines local-first mission, native goal, mission artifact, evidence artifact, mission goal, agentic E2E plan pack, specs phase, native goal brief, goal brief artifact, legacy execution prompt, clarify phase, create PRD phase, plan phase, architecture lens, architecture refactor mission, and execution discipline.
- Current docs still describe $plan as creating EXECUTION_PROMPT.md and instruct users to paste an objective after /goal.
- Current CLI layout still exposes an execution-prompt artifact kind that writes EXECUTION_PROMPT.md.
- Current template registry still has an execution prompt template with goal prompt copy markers.
- Current tests assert execution-prompt descriptors, paths, template docs, and rendered output.
- The repo already has proof, review, triage, and closeout artifact kinds, which should remain evidence-only.
- The repo already documents the anti-oracle boundary: inspect, events, receipts, and setup status are mechanical and not mission truth.
- The user explicitly chose local-first Codex1 missions, whole-mission native goals, evidence-only proof/review/closeout, GOAL_BRIEF.md over EXECUTION_PROMPT.md, no standalone clarification artifact, and Browser-native dogfood out of scope.

<!-- codex1-section: assumptions -->
## Assumptions

- The implementation can rename the current execution-prompt artifact kind to goal-brief without needing to support the old command as a current alias.
- If old missions contain EXECUTION_PROMPT.md, documentation-level legacy reading guidance is sufficient unless planning uncovers a hard compatibility need.
- The existing CLI and tests are small enough that the artifact rename should happen in the same mission rather than as a docs-only first step.
- The exact Rust enum or command names can be chosen during planning, provided user-facing behavior uses goal-brief terminology.
- The root PRD.md that predates this mission should not be overwritten by this PRD.

<!-- codex1-section: resolved_questions -->
## Resolved Questions

- Codex1 should be local-first.
- A native /goal normally completes the whole Codex1 mission plan, not one slice at a time.
- Codex1 keeps proof, review, triage, and closeout as evidence only; native /goal owns completion.
- $plan should produce an agentic E2E plan pack.
- Specs are always considered during planning, but spec files are written only when useful.
- Codex1 should produce a native goal brief that helps Codex create or refine the actual native goal.
- GOAL_BRIEF.md should replace EXECUTION_PROMPT.md in the current artifact model.
- The old EXECUTION_PROMPT.md name should be legacy reading guidance only.
- $clarify remains the user-facing name and follows grill-with-docs behavior.
- $clarify should not create a first-class clarification artifact.
- $create-prd should synthesize and avoid re-interviewing by default.
- $plan may ask questions only when answers change executable shape.
- Improve Codebase Architecture has two modes: architecture lens during clarify/plan and architecture refactor mission.
- TDD is default execution discipline where it fits, with diagnose-first for hard bugs and appropriate proof for non-code changes.
- Browser-native dogfood is out of scope for this redesign.

<!-- codex1-section: implementation_decisions -->
## Implementation Decisions

- Rename the current execution prompt concept to goal brief across current artifact vocabulary.
- Current goal brief artifacts must be named GOAL_BRIEF.md.
- Current CLI output should use goal-brief terminology and should not present EXECUTION_PROMPT.md as a current artifact.
- Docs may mention EXECUTION_PROMPT.md only as legacy reading guidance for older missions.
- Do not add a first-class clarification artifact; clarification side effects remain CONTEXT.md and ADRs.
- Keep proof, review, triage, and closeout artifact kinds as evidence records, not authority.
- Keep local-first PRDs in the Codex1 mission artifact tree.
- Update managed setup skill bodies so newly enabled repos get the redesigned workflow.
- Update the plan guidance to say Codex1 writes GOAL_BRIEF.md as a native goal brief, not as the final sacred prompt.
- Update the goal brief format so it tells Codex enough to create or refine a native mission goal and includes acceptance criteria, execution order, subplan rules, proof rules, review/triage rules, closeout rules, non-completion behavior, and prohibited actions.
- Preserve the native goal boundary: Codex1 must not create, mirror, inspect, or complete native goal state.
- Preserve the anti-oracle boundary: inspect, setup status, events, and receipts must not infer readiness, completion, proof sufficiency, review cleanliness, close safety, or next action.
- Document architecture thinking as both a pre-plan lens and a dedicated refactor mission type.
- Document execution discipline as TDD for behavior/code changes, diagnose-first for hard bugs, and appropriate proof for non-code work.
- Keep Browser-native dogfood out of this mission.

<!-- codex1-section: testing_decisions -->
## Testing Decisions

- Tests should assert user-facing behavior through the CLI, not private implementation details.
- Integration tests should verify mission initialization descriptors use goal-brief terminology and GOAL_BRIEF.md.
- Integration tests should verify template list and template show expose goal-brief terminology.
- Integration tests should verify interviewing the goal-brief artifact writes GOAL_BRIEF.md with the expected native-goal-brief sections.
- Inspect tests should verify goal-brief inventory mechanically without semantic status fields.
- Setup tests should verify managed skills and docs materialize the redesigned workflow language.
- Docs regression tests should catch stale current references to EXECUTION_PROMPT.md while allowing explicitly marked legacy guidance.
- Tests should continue to prove removed or legacy command surfaces do not become current workflow accidentally.
- Existing event, receipt, inspect, path-safety, and anti-oracle tests should continue to pass.
- Do not add brittle tests that assert exact prose unless that prose is part of managed setup output or a required user-facing contract.

<!-- codex1-section: proof_expectations -->
## Proof Expectations

- Run cargo fmt or equivalent formatting checks.
- Run cargo test.
- Run cargo clippy with warnings denied where practical.
- Run targeted CLI smoke checks for init, template list/show, interview goal-brief, inspect, setup status, and setup materialization.
- Search the repository for stale current-use EXECUTION_PROMPT.md and execution-prompt terminology.
- Review generated or managed docs to ensure legacy references are clearly marked as legacy only.
- Record proof artifacts for completed subplans and closeout evidence at the end of the mission.

<!-- codex1-section: review_expectations -->
## Review Expectations

- Review should focus on stale terminology, accidental compatibility duplicates, anti-oracle regressions, setup bundle drift, and whether the goal brief still sounds like a Codex1-owned goal.
- Triage should distinguish true workflow regressions from harmless legacy mentions.
- Closeout should audit PRD success criteria against proof and review evidence without claiming to complete native /goal.

<!-- codex1-section: further_notes -->
## Further Notes

- The product should feel like Codex1 prepares durable context and evidence while native Codex goals do the active work.
- The goal brief should be strong enough that Codex can write a better native goal from it, matching the guidance that Codex often writes better goal prompts for itself.
- Avoid turning this redesign into a dogfood/browser automation mission.
- Avoid asking obvious workflow questions during planning; ask only when answers change executable shape.

<!-- codex1-section: pr_intent -->
## PR Intent

Open a pull request only if the later mission goal explicitly chooses to publish changes; otherwise keep the redesign local-first until the user asks for PR publication.

