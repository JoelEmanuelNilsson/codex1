# Artifact Model

The artifact tree is the durable product. Files are human-facing markdown with minimal frontmatter and deterministic section tags.

Native Codex goals are external thread state. Codex1 artifacts provide context, structure, and evidence for Codex; they are not a continuation scheduler and do not mirror native goal state.

## Mission Artifacts

Clarification artifacts or notes capture the user's raw intent, open questions, constraints, and resolved understanding before PRD synthesis. They are inputs to PRD creation, not execution contracts.

`PRD.md` captures the mission goal, problem statement, solution, behavior-focused user stories, interpreted destination, observable success criteria, boundaries, module sketch, assumptions, implementation and testing decisions, proof expectations, review expectations, and PR intent. Its boundaries distinguish `Always Preserve`, `Ask Before Changing`, and `Out Of Scope` work. It is the anchor for durable work.

`PLAN.md` is an optional route map when a mission genuinely needs one. It can describe execution order, proof strategy, risks, non-goals, and unresolved human decisions. It is not a required phase, status dashboard, dependency graph engine, or proof ledger.

`RESEARCH_PLAN.md` is optional. Codex writes it when research is substantial enough to need durable structure.

`GOAL_BRIEF.md` is an optional native goal brief. It can help Codex create or refine the real native `/goal` for the whole mission by preserving the desired end state, artifacts to read, editable scope, proof/review/triage expectations, completion criteria, non-completion behavior, closeout expectations, and prohibited actions. It is not an execution trigger, native goal state, or necessarily the exact pasteable prompt.

`GOAL_PROMPT.md` is optional. Use it only when the user needs one compact copy source for the exact native `/goal` objective.

`CLOSEOUT.md` summarizes how Codex judges the PRD was satisfied, including completed, superseded, paused, or deferred work and remaining risks. It is durable evidence, not native goal completion state.

## Collection Artifacts

`RESEARCH/` stores research records when research is actually needed: sources inspected, facts found, experiments run, uncertainties, options, recommendations, and affected artifacts.

`SPECS/` stores bounded implementation contracts when the PRD is not precise enough for implementation. Specs describe responsibility, PRD relevance, scope, expected behavior, interfaces, proof expectations, and risks.

`SUBPLANS/` stores tracer-bullet slices in visible lifecycle folders when separate execution contracts are useful. Ready subplans act as durable agent briefs: slice type, current and desired behavior, stable interfaces, scope, out-of-scope work, dependencies, acceptance criteria, proof, and exit criteria. Folder placement is a cue for humans and Codex, not a CLI state machine. Multiple files may be in `active/`.

`ADRS/` stores durable architecture decisions and tradeoffs when a decision is hard to reverse, surprising without context, and has real alternatives. ADRs should stay lightweight unless extra structure adds real value.

`REVIEWS/` stores reviewer opinions. Reviews do not mutate mission truth.

`TRIAGE/` stores main-Codex adjudication of reviews. Triage explains accepted, rejected, deferred, duplicate, or stale findings.

`PROOFS/` stores evidence records for completed subplans: commands, tests, manual checks, changed areas, failures, accepted risks, and links.

## Machine Substrate

The current CLI does not maintain mission-local machine state. Setup metadata lives at the repo level under `.codex1/setup-*`; mission truth remains the human-readable artifact tree.
