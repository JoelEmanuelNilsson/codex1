# Artifact Model

The artifact tree is the durable product. Files are human-facing markdown with minimal frontmatter and deterministic section tags.

Native Codex goals are external thread state. Codex1 artifacts provide context, structure, and evidence for Codex; they are not a continuation scheduler and do not mirror native goal state.

## Mission Artifacts

Clarification artifacts or notes capture the user's raw intent, open questions, constraints, and resolved understanding before PRD synthesis. They are inputs to PRD creation, not execution contracts.

`PRD.md` captures the mission goal, problem statement, solution, user stories, interpreted destination, success criteria, module sketch, constraints, assumptions, implementation and testing decisions, proof expectations, review expectations, and PR intent. It is the anchor for durable work.

`PLAN.md` is the executable route map. It describes the outcome contract, implementation shape, execution order, useful parallelization notes, ready subplans, proof strategy, risks, non-goals, and unresolved human decisions if any. It is not a status dashboard, dependency graph engine, or proof ledger.

`RESEARCH_PLAN.md` is optional. Codex writes it when research is substantial enough to need durable structure.

`GOAL_BRIEF.md` is the native goal brief produced by planning. It helps Codex create or refine the real native `/goal` for the whole mission. It tells Codex what mission to execute, which artifacts to read, how to select subplans, how workers may be used, what may be edited, how proofs/reviews/triage should be recorded, what completion means, what to record if completion cannot be reached, what closeout means, and what not to do. It is not an execution trigger or native goal state by itself.

`CLOSEOUT.md` summarizes how Codex judges the PRD was satisfied, including completed, superseded, paused, or deferred work and remaining risks. It is durable evidence, not native goal completion state.

## Collection Artifacts

`RESEARCH/` stores research records: sources inspected, facts found, experiments run, uncertainties, options, recommendations, and affected artifacts.

`SPECS/` stores bounded implementation contracts. Specs describe responsibility, PRD relevance, scope, expected behavior, interfaces, proof expectations, and risks.

`SUBPLANS/` stores tracer-bullet slices in visible lifecycle folders. Ready subplans act as durable agent briefs: slice type, current and desired behavior, stable interfaces, scope, out-of-scope work, dependencies, acceptance criteria, proof, and exit criteria. Folder placement is a cue for humans and Codex, not a CLI state machine. Multiple files may be in `active/`.

`ADRS/` stores durable architecture decisions and tradeoffs. ADRs should stay lightweight unless extra structure adds real value.

`REVIEWS/` stores reviewer opinions. Reviews do not mutate mission truth.

`TRIAGE/` stores main-Codex adjudication of reviews. Triage explains accepted, rejected, deferred, duplicate, or stale findings.

`PROOFS/` stores evidence records for completed subplans: commands, tests, manual checks, changed areas, failures, accepted risks, and links.

## Machine Substrate

The current CLI does not maintain mission-local machine state. Setup metadata lives at the repo level under `.codex1/setup-*`; mission truth remains the human-readable artifact tree.
