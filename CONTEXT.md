# Codex1

Codex1 is a local artifact workflow for making Codex missions understandable, executable, and auditable without becoming the active goal engine.

## Language

**Local-first mission**:
A Codex1 mission whose durable planning and evidence live in the repository, with issue trackers used only when the user explicitly wants external publishing or sharing.
_Avoid_: Issue-first mission, tracker-owned mission

**Native goal**:
The Codex-owned persistent objective created through `/goal`; it owns continuation, pause/resume, accounting, and completion.
_Avoid_: Codex1 goal, mission state

**Mission artifact**:
A durable Codex1 file that preserves intent, plans, specs, evidence, or handoff context for a mission.
_Avoid_: Goal state, status oracle

**Evidence artifact**:
A mission artifact that records proof, review, triage, or closeout evidence without owning completion authority.
_Avoid_: Completion gate, goal status

**Mission goal**:
A native goal whose normal job is to complete one whole Codex1 mission plan, using ready subplans as executable slices.
_Avoid_: Slice goal by default, Codex1-owned goal

**Agentic E2E plan pack**:
The normal Codex1 planning output: a mission plan, acceptance criteria, executable subplans, proof expectations, optional specs, and a native-goal brief strong enough for Codex to run the mission end to end.
_Avoid_: Vague plan, prompt-only plan

**Specs phase**:
The planning checkpoint where Codex decides which exact behavior, interface, artifact, browser-flow, or test contracts need dedicated specs, and records when no dedicated specs are needed.
_Avoid_: Mandatory spec ceremony, skipped contract thinking

**Native goal brief**:
The Codex1 planning artifact that gives Codex enough mission context, acceptance criteria, execution rules, proof rules, and closeout rules to create or refine the actual native goal.
_Avoid_: Execution prompt, sacred final prompt, Codex1-owned goal

**Goal brief artifact**:
The canonical mission artifact named `GOAL_BRIEF.md`; it replaces the older `EXECUTION_PROMPT.md` concept in the CLI artifact model, templates, docs, and tests.
_Avoid_: EXECUTION_PROMPT.md

**Artifact catalog**:
The current Codex1 module that should own artifact ids, titles, paths, and template identity for generated mission artifacts.
_Avoid_: Legacy alias catalog, scattered artifact vocabulary

**Legacy execution prompt**:
The old `EXECUTION_PROMPT.md` artifact name, which may appear in older missions only as legacy reading guidance.
_Avoid_: Current generated artifact, compatibility duplicate

**Clarify phase**:
The Codex1 discovery phase that challenges the user's idea against docs, code, domain language, and ADR-worthy tradeoffs before PRD synthesis.
_Avoid_: Generic Q&A, implementation planning, standalone clarification artifact

**Create PRD phase**:
The Codex1 synthesis phase that turns known clarification context and repo evidence into a local PRD, recording assumptions instead of restarting the interview.
_Avoid_: Second interview, issue publishing

**Plan phase**:
The Codex1 design phase that turns a PRD into an agentic E2E plan pack, asking only when an answer changes executable shape.
_Avoid_: Second clarify session, vague task list

**Architecture lens**:
The use of architecture-review thinking during clarify or planning to find seams, deep modules, testability risks, and ADR conflicts before execution slices are written.
_Avoid_: Refactor-only architecture

**Architecture refactor mission**:
A mission whose direct purpose is to deepen modules, improve locality, simplify interfaces, or make the codebase more agent-navigable and testable.
_Avoid_: Incidental cleanup

**Execution discipline**:
The default implementation posture for ready subplans: use TDD for behavior and code changes, diagnose-first for hard bugs, and fit proof to the actual artifact when TDD would be fake ceremony.
_Avoid_: Mandatory TDD theater, unproven implementation

## Relationships

- A **Local-first mission** may produce many **Mission artifacts**.
- A **Native goal** may use **Mission artifacts** as context and evidence.
- A **Mission artifact** does not create, mirror, inspect, or complete a **Native goal**.
- A **Mission goal** executes a whole Codex1 mission plan by completing or explicitly accounting for its ready subplans.
- An **Evidence artifact** supports Codex's judgment but does not decide readiness, correctness, or completion.
- An **Agentic E2E plan pack** gives a **Mission goal** enough context, scope, and proof criteria to execute without asking more planning questions.
- A **Specs phase** may produce specs, but it may also explicitly decide that subplan acceptance criteria are sufficient.
- A **Native goal brief** is input to native goal creation; it is not itself the native goal.
- The **Goal brief artifact** is the durable file representation of the **Native goal brief**.
- A **Legacy execution prompt** may be interpreted as an old goal brief, but current Codex1 commands should generate only the **Goal brief artifact**.
- The **Artifact catalog** should describe current generated artifacts only; legacy execution-prompt names belong in documentation, not current command aliases.
- The **Clarify phase** uses the grill-with-docs discipline while keeping the user-facing Codex1 name `$clarify`; it updates `CONTEXT.md` and ADRs inline rather than creating a dedicated clarification artifact.
- The **Create PRD phase** asks only when trusted sources contradict each other or a missing answer would make the PRD misleading.
- The **Plan phase** uses `AFK` and `HITL` labels to keep unresolved human decisions out of ready execution.
- The **Architecture lens** can inform a normal mission plan without making architecture refactoring the whole mission.
- An **Architecture refactor mission** uses the same Codex1 clarify, PRD, plan, goal brief, evidence, and closeout flow as feature work.
- **Execution discipline** requires test-first behavior work where it fits, but uses documentation review, CLI output, snapshot checks, or other appropriate proof for non-code artifacts.

## Flagged ambiguities

- Browser-native dogfood is intentionally out of scope for the current Codex1 workflow redesign.
