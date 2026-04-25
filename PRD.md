# Codex1 PRD: Built-In Template Artifact Workflow For Native Codex

## Problem Statement

Codex is already capable of reasoning, coding, reviewing, using tools, and coordinating subagents. The problem is not that Codex needs a replacement runtime. The problem is that large, risky, ambiguous, or long-running coding missions exceed what ordinary chat memory and informal notes can reliably hold.

The user wants Codex to be able to take a serious goal, clarify it, research it, plan it, specify it, delegate parts of it, review it, revise it, prove it, and close it without losing the plot. The user also wants this to feel native to Codex: skills and normal Codex turns remain the experience, not a separate application that traps the user inside a project-management system.

The previous Codex1 direction had the right intent but drifted into the wrong authority model. The CLI became a semantic workflow oracle. It tried to decide whether work was ready, whether reviews passed, whether close was safe, whether replan was required, whether proof was fresh, whether graph waves were safe, and whether terminal completion was valid. Code reviews repeatedly found new edge cases because this kind of CLI has to maintain a complete semantic state machine for an open-ended agent workflow. That is too much authority for a deterministic helper tool.

The revised problem is therefore:

- Codex needs durable, high-quality artifacts for serious work.
- Codex needs deterministic help creating those artifacts.
- Codex needs a minimal continuation guard so it does not accidentally stop during explicit execute/autopilot loops.
- Codex does not need a smart CLI that decides mission truth.
- The user does not want bloated process, hidden state machines, fake subagent permissions, or a review-finding generator disguised as a product.

Codex1 must preserve the original dream: make Codex more powerful by giving it rails. But those rails must be simple, visible, and artifact-centered.

## Solution

Codex1 will be rebuilt as a built-in-template artifact workflow for native Codex.

From the user's perspective, Codex1 is not an app they operate directly. The user invokes Codex skills such as clarify, plan, execute, review-loop, interrupt, and autopilot. Those skills use a deterministic `codex1` CLI to conduct structured interviews, scaffold high-quality artifacts, record evidence, and manage a tiny explicit continuation loop for Ralph.

The central product idea is:

- Codex thinks.
- Codex judges.
- Codex researches.
- Codex decides what is semantically true.
- The CLI asks deterministic questions and writes durable artifacts.
- The CLI validates mechanical shape, not semantic truth.
- Ralph only nudges Codex to continue when Codex explicitly requested continuation.

Every durable mission begins with a PRD artifact. The PRD is the heart of the mission. It captures what the user wants, the interpreted destination, success criteria, constraints, non-goals, relevant verified context, assumptions, proof expectations, review expectations, and PR intent. The PRD can be short for small missions or very deep for large missions. The CLI enforces that required sections exist; Codex decides how much detail is appropriate.

The plan artifact is a living strategy map. It describes the route from the PRD to completion: strategy, phases, workstreams, sequencing, risks, research needs, relevant specs, relevant subplans, review posture, and the next slice. The plan is not a status oracle, task tracker, or graph scheduler.

Research artifacts capture what Codex learned before deciding how to proceed. For large or uncertain missions, the plan skill may first create a research plan, run research, record research artifacts, and only then create or revise the main plan.

Specs are spec-driven development contracts for bounded implementation responsibilities, often used by subagents. Specs describe what to build, why it matters to the PRD, what is in scope, what is out of scope, expected behavior, relevant constraints, expected proof, and risks.

Subplans are executable slices. Every executable slice has a subplan. Subplans live in lifecycle folders such as ready, active, done, paused, and superseded. The filesystem placement is a visible signal for Codex and humans, not a semantic state machine enforced by the CLI.

ADRs preserve durable architecture decisions. Reviews preserve reviewer opinions. Triage records preserve main Codex's adjudication of reviews. Proofs record evidence for completed subplans. Closeout summarizes how the PRD was satisfied, what was completed, what was superseded or deferred, what proofs exist, what reviews happened, what risks remain, and whether PR work is ready or already done.

The CLI provides built-in, versioned templates only. There are no user-editable templates in the first product. This keeps artifact quality deterministic and prevents template customization from becoming another truth surface.

The CLI supports interactive interviews and equivalent non-interactive answers-file flows. Interactive mode helps Codex answer one structured question at a time. Answers-file mode supports tests, automation, retries, and reproducibility.

The CLI may inspect and summarize artifact presence. It may check parseability, required fields, safe paths, unique IDs, and stable output envelopes. It must not decide whether the plan is good, whether a task is ready, whether review passed, whether proof is sufficient, whether close is safe, or whether replan is required.

Ralph is reduced to a minimal Stop-hook adapter over explicit loop state. Ralph does not infer readiness from PRD, plan, specs, subplans, reviews, or proofs. If an explicit loop file says continuation is active and unpaused, Ralph blocks once with the recorded message and tells Codex how to pause or stop the loop. Otherwise Ralph allows stop.

## User Stories

1. As a user, I want every durable mission to begin with a PRD, so that Codex does not drift away from my actual goal.
2. As a user, I want the PRD to be as short or as detailed as the mission demands, so that small work stays light and serious work gets enough rigor.
3. As a user, I want Codex to ask clarification questions before writing the PRD when needed, so that the PRD does not silently encode wrong assumptions.
4. As a user, I want Codex to create a PRD through a deterministic interview, so that important sections are not forgotten.
5. As a user, I want Codex to keep the workflow native to Codex skills, so that I am not forced into a separate project-management interface.
6. As a user, I want the CLI to help Codex create good artifacts, so that Codex does not have to hand-write perfect markdown structure from memory.
7. As a user, I want built-in templates only, so that artifact quality is consistent and not dependent on local template drift.
8. As a user, I want the CLI to avoid semantic judgments, so that it does not become an unreliable workflow authority.
9. As a user, I want Codex to remain the judge of what satisfies the PRD, so that a deterministic tool does not make brittle product decisions.
10. As a user, I want a plan that explains the strategy and route, so that I can understand how Codex intends to reach the PRD.
11. As a user, I want the plan to remain readable and current, so that it does not become an append-only museum.
12. As a user, I want the plan to avoid becoming a giant task tracker, so that it stays useful for strategy rather than ceremony.
13. As a user, I want huge goals to be decomposed into phases and workstreams, so that massive efforts are understandable.
14. As a user, I want uncertain missions to have research before final planning, so that Codex does not pretend to know what it has not learned.
15. As a user, I want substantial research to be captured durably, so that future Codex sessions can see what was inspected and learned.
16. As a user, I want the plan skill to decide whether research is needed, so that I do not need to pick a separate planning mode manually.
17. As a user, I want research plans when research is substantial, so that large unknowns are investigated intentionally.
18. As a user, I want research artifacts to capture sources, facts, experiments, and recommendations, so that decisions have evidence.
19. As a user, I want specs for implementation responsibilities, so that subagents receive crisp contracts.
20. As a user, I want specs to be created whenever they are useful, so that planning can be deep without being rigid.
21. As a user, I want specs to be modifiable when coding reveals new facts, so that the workflow adapts to reality.
22. As a user, I want workers to receive the PRD, plan, and relevant spec, so that local implementation work stays aligned with the whole mission.
23. As a user, I want subagents to understand the whole mission by default, so that they do not optimize locally against the wrong goal.
24. As a user, I want subagents to be prevented by instruction from editing the PRD or plan, so that mission truth remains main-thread owned.
25. As a user, I want subagents to be allowed to edit their assigned spec when explicitly allowed, so that implementation discoveries can improve the contract.
26. As a user, I want subagents to propose PRD or plan changes rather than apply them directly, so that main Codex can preserve coherence.
27. As a user, I want every executable slice to have a subplan, so that execution always has a clear unit of work.
28. As a user, I want subplans to be lightweight when the work is small, so that determinism does not become bloat.
29. As a user, I want subplans to be detailed when the work is large, so that complex implementation can be delegated safely.
30. As a user, I want subplans to live in visible lifecycle folders, so that I can see what is ready, active, done, paused, or superseded.
31. As a user, I want multiple active subplans to be allowed, so that Codex can choose parallel work when appropriate.
32. As a user, I want the skill guidance to teach Codex how to reason about parallelism, so that the CLI does not need to approve parallel waves.
33. As a user, I want reviews to be timestamped records, so that reviewer opinions are preserved without becoming executable state.
34. As a user, I want triage records to capture what main Codex accepted or rejected from reviews, so that review findings do not automatically become work.
35. As a user, I want proof artifacts for completed subplans, so that completion claims are backed by evidence.
36. As a user, I want one proof per completed subplan, so that evidence aligns with executable slices.
37. As a user, I want proof records to include tests, commands, manual checks, changed areas, and residual risks, so that future Codex can understand the evidence.
38. As a user, I want closeout to summarize PRD satisfaction rather than mechanically require every historical subplan to be done, so that superseded or paused work can be explained.
39. As a user, I want closeout to list completed, superseded, paused, and deferred work, so that the final state is honest.
40. As a user, I want ADRs for important architectural decisions, so that durable tradeoffs do not get buried in mutable plan prose.
41. As a user, I want the CLI to run interactive interviews, so that Codex can answer structured prompts step by step.
42. As a user, I want every interactive interview to have a non-interactive answers-file equivalent, so that tests and automation can use the same behavior.
43. As a user, I want the CLI to write deterministic markdown from answers, so that artifacts are consistent and easy to review.
44. As a user, I want the CLI to validate only mechanical form, so that validation remains reliable.
45. As a user, I want the CLI to reject unsafe mission IDs and path escapes, so that generated artifacts stay inside the mission.
46. As a user, I want the CLI to return stable JSON envelopes when requested, so that Codex can use it predictably.
47. As a user, I want inspection commands to report artifact inventory, so that Codex can quickly see what exists.
48. As a user, I do not want inspection commands to claim semantic readiness, so that they do not become another oracle.
49. As a user, I want a tiny explicit loop state for Ralph, so that continuation pressure is intentional.
50. As a user, I want Ralph to block only when Codex explicitly started a continuation loop, so that it does not fight normal conversation.
51. As a user, I want Ralph's block message to include the pause command, so that Codex can exit the loop when continuation is wrong.
52. As a user, I want interrupt to pause the loop, so that I can talk to Codex without being forced back into execution.
53. As a user, I want autopilot to clarify before planning, so that it does not replace missing user intent with assumptions.
54. As a user, I want autopilot to create a PRD, plan, specs, subplans, reviews, proofs, and closeout as appropriate, so that a serious mission can be carried end to end.
55. As a user, I want autopilot not to open a PR unless the PRD says to do so, so that external actions remain intentional.
56. As a user, I want execute to work from active or ready subplans, so that execution is slice-based rather than a vague whole-plan push.
57. As a user, I want execute to create or update specs when implementation discoveries require it, so that the artifacts reflect reality.
58. As a user, I want review-loop to create reviews and triage records, so that iterative critique is structured but not automatic authority.
59. As a user, I want reviewer agents to record opinions through the CLI, so that their output is shaped consistently.
60. As a user, I want reviewer agents not to mutate mission truth, so that findings remain opinions until triaged by main Codex.
61. As main Codex, I want deterministic interviews for each artifact type, so that I can produce high-quality artifacts without remembering every section.
62. As main Codex, I want artifact templates to encode best-practice questions, so that plans and specs become better by default.
63. As main Codex, I want to decide when research is enough, so that I can balance speed and rigor.
64. As main Codex, I want to update the plan as new facts emerge, so that the strategy remains current.
65. As main Codex, I want to move subplans between lifecycle folders, so that execution progress is visible without a hidden state machine.
66. As main Codex, I want to create proofs after completing subplans, so that I can justify completion.
67. As main Codex, I want to create a closeout when the PRD is satisfied, so that final state is explained in human terms.
68. As a worker subagent, I want to receive PRD, plan, and my relevant spec/subplan, so that I understand both global context and local responsibility.
69. As a worker subagent, I want a clear assigned scope, so that I know what to edit and what not to touch.
70. As a worker subagent, I want to report conflicts between the spec and codebase, so that main Codex can revise artifacts before bad implementation proceeds.
71. As a reviewer subagent, I want a structured review recording flow, so that my findings include priority, confidence, location, and rationale.
72. As a reviewer subagent, I want my review to be preserved as an opinion record, so that main Codex can triage it.
73. As a future Codex session, I want to read PRD, plan, research, specs, subplans, ADRs, reviews, triage, proofs, and closeout, so that I can resume with context.
74. As a future Codex session, I want artifact placement to be obvious, so that I do not need to reverse engineer hidden state.
75. As a maintainer, I want the product to avoid a semantic state machine, so that code reviews do not endlessly find workflow edge cases.
76. As a maintainer, I want templates to be built-in and versioned, so that the product can evolve predictably.
77. As a maintainer, I want the implementation to have deep modules with simple interfaces, so that artifact rendering, interviews, paths, and loop handling can be tested independently.
78. As a tester, I want to validate generated artifacts from known answers, so that template behavior is deterministic.
79. As a tester, I want to validate Ralph with explicit loop files only, so that stop-hook behavior stays tiny and reliable.
80. As a tester, I want to confirm the CLI never emits semantic readiness claims, so that oracle behavior does not creep back in.

## Implementation Decisions

- Codex1 is a skills-first product. The user-facing experience is Codex skills using the CLI, not the CLI as a standalone project-management application.
- The CLI is a deterministic artifact interviewer, scaffolder, recorder, inspector, and loop helper.
- The CLI is not a semantic workflow engine.
- The CLI must never decide whether a task is ready, whether review passed, whether close is safe, whether a plan is good, whether proof is sufficient, or whether replan is required.
- Every durable mission requires a PRD artifact.
- The clarify skill always creates the PRD artifact.
- The PRD artifact replaces the older outcome concept.
- PRD confirmation is evidence or context, not a CLI ratification authority.
- The plan skill may proceed when Codex judges the PRD is sufficient.
- Built-in templates are the only templates in the first product.
- User-editable templates, project template overrides, and template plugins are out of scope for the first product.
- Artifact templates are versioned.
- Artifact templates are markdown-first.
- Human-facing artifacts use markdown with minimal metadata.
- Machine substrate is limited to explicit loop state and optional audit receipts.
- No authoritative semantic state file is included in the first product.
- If a cache or index is ever added later, it must be disposable and not contractual truth.
- Audit receipts are optional receipts, not replay authority.
- There is no event replay engine in the first product.
- The artifact tree is the primary durable context.
- The PRD artifact captures the user's goal, interpreted destination, success criteria, non-goals, constraints, verified context, assumptions, resolved questions, proof expectations, review expectations, and PR intent.
- The PRD can be brief or extensive depending on mission risk and context.
- The plan artifact is a living strategy map.
- The plan artifact captures strategy, workstreams, phases, sequencing, risk posture, research needs, artifact index, review posture, and current recommended slices.
- The plan artifact is mutable and current.
- The plan artifact is not append-only.
- The plan artifact is not a task tracker, status dashboard, graph scheduler, or proof ledger.
- The plan may include all major steps required to achieve the PRD, but at program-plan altitude rather than microtask altitude.
- A research plan artifact is created only when research is substantial enough to need durable structure.
- The plan skill owns research planning and research execution as part of planning.
- There is no separate user-facing research-planning skill in the first product.
- Research artifacts capture questions, sources inspected, facts found, experiments run, uncertainties, options, and recommendations.
- The plan may begin as a research-first strategy and later be updated after research.
- Specs are spec-driven development contracts for bounded implementation responsibilities.
- Specs can be created during planning, execution, or after discoveries.
- Specs can be modified when facts change.
- Subagents may modify their own assigned spec only when explicitly allowed.
- Subagents may propose changes to the PRD, plan, or other artifacts.
- Subagents do not directly edit the PRD or plan.
- Every executable slice has a subplan.
- Subplans are executable slice plans.
- Subplans may be very small or very detailed depending on the slice.
- Subplans live in lifecycle folders for ready, active, done, paused, and superseded work.
- Multiple active subplans are allowed.
- The CLI does not decide when parallel work is safe.
- Skill instructions teach Codex how to reason about parallel work, DAG planning, write ownership, risk, and subagent coordination.
- The filesystem lifecycle of subplans is a visible cue, not a CLI-enforced semantic workflow.
- Specs remain flat or simply organized by artifact identity rather than lifecycle folders.
- Reviews are timestamped opinion records.
- Triage records are timestamped main-Codex adjudication records.
- Reviews and triage do not use lifecycle folders.
- Every completed subplan should have a matching proof artifact.
- Proof artifacts record the subplan/spec satisfied, commands run, tests run, manual checks, changed areas, failures, accepted residual risks, and evidence notes.
- Closeout is created when Codex judges the PRD is satisfied.
- Closeout explains completed subplans, superseded subplans, paused or deferred subplans, proofs, reviews, triage, known risks, and PR readiness.
- Closeout is not gated by a CLI oracle.
- ADR artifacts are first-class for durable architecture decisions and tradeoffs.
- The CLI provides interactive interview flows for PRD, research plan, research, plan, spec, subplan, ADR, review, triage, proof, and closeout.
- Each interview flow has a non-interactive answers-file equivalent.
- Interactive interviews ask deterministic questions.
- Answers are inserted into deterministic tagged sections.
- The CLI validates that required answers exist and that the generated artifact is structurally valid.
- The CLI rejects unsafe mission IDs and path escapes.
- The CLI uses stable success and error envelopes for machine-readable mode.
- The CLI provides an inspect command that reports artifact inventory and basic mechanical warnings.
- The inspect command must not report semantic readiness.
- The inspect command must not infer next task, review pass, close readiness, or PRD satisfaction.
- Ralph uses only explicit loop state.
- Explicit loop state contains active, paused, mode, message, and pause or stop command guidance.
- Ralph allows stop when loop state is missing, corrupt, inactive, paused, or lacks a continuation message.
- Ralph blocks at most once per Stop-hook continuation cycle.
- Ralph block messages include the recorded continuation message and the command to pause or stop.
- Ralph does not read PRD, plan, specs, subplans, reviews, triage, proofs, or closeout.
- The loop commands start, pause, resume, stop, and inspect explicit continuation state.
- Interrupt pauses loop state and returns the user to discussion mode.
- Autopilot runs clarify, plan, execute, review when useful, proof, and closeout.
- Autopilot may open a PR only when PR creation is part of the PRD.
- Execute works from active or ready subplans.
- Execute may create or revise specs when implementation discoveries make that necessary.
- Review-loop records review artifacts and triage artifacts rather than making reviews automatic authority.
- Reviewer subagents may use the CLI to record structured opinions.
- Main Codex owns triage.
- Workers receive PRD, plan, relevant spec, relevant subplan, applicable ADRs, scope, proof expectations, and non-goals.
- Workers do not mutate mission-level artifacts unless explicitly assigned.
- The product should include a companion skill set, but this PRD focuses on the CLI and artifact model needed by those skills.
- The implementation should have deep modules with simple, testable interfaces.
- The artifact template engine should be isolated from command parsing.
- The interview engine should be isolated from artifact rendering.
- Mission path safety should be isolated and heavily tested.
- Loop/Ralph behavior should be isolated and tiny.
- Inspection should be isolated from semantic workflow decisions.
- Doctor checks should verify the installed command can run from outside the source checkout when installation exists.

## Testing Decisions

- Tests should verify external behavior, generated artifacts, command envelopes, and file effects.
- Tests should not rely on private implementation details.
- Template rendering should be tested with golden outputs generated from fixed answers.
- Every built-in template should have at least one complete-answer test and one missing-required-answer test.
- PRD interview tests should verify that all required sections are created and filled from answers.
- Plan interview tests should verify that strategy, phases, workstreams, risks, artifact index, and recommended slices are rendered deterministically.
- Research plan tests should verify that research questions, sources to inspect, expected outputs, and completion criteria are rendered deterministically.
- Research artifact tests should verify that inspected sources, facts, experiments, uncertainties, and recommendations are captured.
- Spec interview tests should verify that implementation contracts include scope, non-goals, behavior, interfaces, proof expectations, and risks.
- Subplan interview tests should verify that executable slices include goal, scope, steps, ownership, linked specs, proof expectations, and exit criteria.
- ADR interview tests should verify that decision, context, options, tradeoffs, consequences, and status are captured.
- Review interview tests should verify that overall assessment, confidence, findings, priorities, locations, and rationale are captured.
- Triage interview tests should verify that accepted, rejected, deferred, duplicate, and stale findings are recorded with reasons.
- Proof interview tests should verify that evidence aligns with a completed subplan and records commands, tests, manual checks, changed areas, and risks.
- Closeout interview tests should verify that completed, superseded, paused, deferred, proof, review, triage, risk, and PR readiness sections are present.
- Answers-file mode should be tested for every interview that supports interactive mode.
- Interactive mode should be tested at the command boundary with simulated input, but business logic should be tested through the shared interview engine.
- Stable JSON envelope tests should cover success and error output.
- Path safety tests should cover mission IDs, path traversal, absolute paths, symlinks, and artifact writes.
- Artifact writes should be tested for atomicity where practical.
- Inspect command tests should verify artifact inventory and mechanical warnings.
- Inspect command tests should verify that no semantic readiness claims are emitted.
- Loop command tests should verify start, pause, resume, stop, and status behavior.
- Ralph tests should verify missing loop state allows stop.
- Ralph tests should verify corrupt loop state allows stop.
- Ralph tests should verify inactive loop allows stop.
- Ralph tests should verify paused loop allows stop.
- Ralph tests should verify active unpaused loop with message blocks once.
- Ralph tests should verify Stop-hook active circuit breaker allows stop.
- Ralph tests should verify block messages include pause or stop guidance.
- Doctor tests should verify installed-command behavior without relying on source-local execution.
- Regression tests should explicitly prevent reintroducing semantic oracle behavior into inspect or loop commands.
- Tests should assert that there is no authoritative semantic state file in the initial product.
- Tests should assert that audit receipts are not required to determine artifact inventory.
- Tests should assert that subplan lifecycle movement is a file operation, not a semantic approval.
- Tests should assert that multiple active subplans are allowed.
- Tests should assert that moving a subplan to done does not require the CLI to prove the work is correct.
- Tests should assert that proof creation is possible for a done subplan but does not cause the CLI to claim PRD satisfaction.
- Tests should assert that closeout creation is a structured artifact write, not a semantic close gate.
- The implementation should include end-to-end tests for a small mission: create PRD, create plan, create subplan, move subplan active, create proof, move subplan done, create closeout.
- The implementation should include end-to-end tests for a research-heavy mission: create PRD, create research plan, create research artifacts, create plan, create spec, create subplan.
- The implementation should include end-to-end tests for a review loop: create review, create triage, revise spec or plan through explicit artifact rewrite.
- The implementation should include end-to-end tests for Ralph: start loop, trigger hook block, pause loop, trigger hook allow.
- Tests should not require a GitHub issue, network service, external model, or actual Codex subagent.

## Out of Scope

- Building a wrapper runtime around Codex.
- Building a smart status oracle.
- Building authoritative semantic state.
- Building event replay as mission authority.
- Computing next tasks.
- Computing graph waves.
- Enforcing DAG readiness.
- Deciding review pass or fail.
- Deciding proof sufficiency.
- Deciding close readiness.
- Deciding PRD satisfaction.
- Enforcing semantic replan rules.
- Building a project-management dashboard.
- User-editable templates.
- Project-specific template overrides.
- Template plugins.
- Hidden daemons.
- Fake subagent permission systems.
- Caller identity detection.
- Session authority tokens.
- Reviewer writeback authority.
- Public finish or complete skills.
- A CLI that asks open-ended semantic clarification questions on its own.
- A CLI that spawns subagents.
- A CLI that opens PRs.
- GitHub issue creation.
- Network integrations.
- Model-provider integrations.
- Complex terminal UI.
- A database.
- Event sourcing or replay repair.
- Mandatory review loops for all missions.
- Mandatory research plans for small missions.
- Mandatory specs for trivial one-shot non-durable work.
- Any feature that makes the CLI the judge of mission truth.

## Further Notes

The essential redesign is an authority inversion.

The abandoned architecture made the CLI answer semantic questions:

- Is the mission complete?
- Is this task ready?
- Is this review clean?
- Is this close safe?
- Is replan required?
- Is this proof fresh enough?

Those questions generated endless edge cases because they require judgment. Codex can make those judgments. A deterministic CLI should not.

The new architecture asks the CLI to answer mechanical questions:

- Does this artifact exist?
- Can this answer set render a template?
- Is this required section filled?
- Is this mission path safe?
- Did this write happen?
- Is this loop explicitly active?
- Is this command output stable JSON?

Those questions are appropriate for a CLI.

The product should feel like Codex has a world-class mission notebook and artifact factory. It should not feel like Codex is fighting a workflow database.

The first implementation should be boring. It should prove:

- built-in templates work
- interviews work
- artifacts are generated deterministically
- paths are safe
- inspect is inventory-only
- loop state is tiny
- Ralph is tiny
- no semantic state machine exists

Once that foundation works, skills can make the experience powerful. The skill layer can teach Codex how to use PRD, research, plan, specs, subplans, reviews, triage, proofs, closeout, and Ralph to execute serious missions.

The most important invariant is:

Codex1 preserves and structures Codex's thinking; it does not replace that thinking.
