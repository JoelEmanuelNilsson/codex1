---
artifact: outcome-lock
mission_id: contract-centered-architecture
root_mission_id: contract-centered-architecture
parent_mission_id: null
version: 1
lock_revision: 1
status: locked
lock_posture: unconstrained
slug: contract-centered-architecture
---
# Outcome Lock

## Objective

Carry Codex1 all the way to a native, skills-first, spec-driven, Ralph-governed end state where a user can enter a Codex session, clarify intent until the mission is genuinely clear, obtain state-of-the-art execution-ready plans, and then rely on Codex1 to execute autonomously until the intended goal is truly done.

## Done-When Criteria

- `clarify` asks high-leverage questions until the destination is genuinely clear, with a user experience strong enough that the user knows what context to provide and how to answer.
- `plan` produces the strongest practical spec-driven planning system for this repo, including clear umbrella planning, bounded `SPEC.md` slices, and explicit sub-agent orchestration contracts.
- `execute` and `autopilot` can continue under Ralph until the mission is truly done, and PRD-based repo review across five review sub-agents yields no agreed `P0`, `P1`, or `P2` findings.

## Success Measures

- The end-to-end user flows for `clarify`, `plan`, `execute`, and `autopilot` all work as intended for the Codex1 product.
- The repo is strong enough that five review sub-agents reviewing against the PRD do not surface any `P0`, `P1`, or `P2` findings that you agree with.
- If recurring review findings persist during execution, Codex1 steps back, identifies the recurring theme, and rethinks or replans the underlying contract or architecture rather than patching forever at the edges.

## Protected Surfaces

- Keep the public skills surface, including `clarify`, `plan`, `execute`, `review`, and `autopilot`; existing skills may be improved aggressively but not removed.
- Keep Codex1 native to Codex and skills-first rather than rebuilding a wrapper-runtime control plane.
- Keep planning and execution spec-driven, with clear umbrella route truth and clear bounded specs or instructions for subordinate work.

## Unacceptable Tradeoffs

- Shipping a hacky, over-engineered, or bug-prone system that repeats the same reliability problems in a cleaner-looking shape.
- Weakening the product into a partial workflow where clarify, planning, Ralph continuation, or sub-agent orchestration are not genuinely trustworthy.
- Declaring the mission done while PRD-based review still surfaces agreed `P0`, `P1`, or `P2` findings.

## Non-Goals

- Perfect agreement with every review sub-agent on every finding, especially when a complaint is weaker than the parent mission truth or stronger repo context.
- Eliminating every possible low-severity comment or aesthetic disagreement once the real product proof bar is met.
- Preserving the current form of any skill, schema, or helper surface merely for familiarity when a stronger design is needed.

## Autonomy Boundary

- Codex may decide later without asking: during planning, repo writes, commits, PR creation when useful, sub-agent orchestration, tests, and relevant external-service work needed for planning; after clarify and planning are complete, `execute` and `autopilot` may take any action autonomously, including irreversible or high-risk actions, external side effects, deploys, migrations, schema changes, deletes, force-pushes, and production writes, so long as those actions serve the locked mission.
- Codex must ask before deciding: any change to the locked objective, protected surfaces, unacceptable tradeoffs, non-goals, or the fact that this mission is the umbrella mission.

## Locked Field Discipline

The fields above for objective, done-when criteria, protected surfaces, unacceptable tradeoffs, non-goals, autonomy boundary, and reopen conditions are locked fields. Change them only through an explicit reopen or superseding lock revision, never by silent mutation.

Baseline facts and rollout or migration constraints are also revision-gated: extend them only through an explicit lock revision when new truth materially changes the destination contract.

## Baseline Current Facts

- The repo already contains deterministic internal commands for clarify bootstrap, planning writeback, execution packages, review bundles, contradictions, resume, and validation.
- Current mission truth and machine legality are still distributed across visible artifacts plus hidden Ralph files such as `state.json`, `active-cycle.json`, `closeouts.ndjson`, and `gates.json`.
- The mission is the umbrella Codex1 mission and owns the full end state, even when later child missions are used as bounded execution structure.

## Rollout Or Migration Constraints

- The umbrella mission may decompose into later child missions or bounded specs, but those slices remain subordinate to this umbrella destination contract.
- Planning should stay bounded and route-defining; it should not itself perform deploys, migrations, or destructive execution steps.
- Execution and autopilot should be designed around the approved fully autonomous execution authority after clarify and planning are complete.

## Remaining Low-Impact Assumptions

- `Perfect` means the strongest practical Codex1 workflow and planning product we can honestly ship in this repo, not a claim of mathematical optimality.
- The five-reviewer cleanliness bar is evaluated using your agreement with the findings, not raw reviewer disagreement alone.
- AGENTS.md-related complaints that you judge weaker than stronger repo truth do not by themselves block completion.

## Reopen Conditions

- The desired product claim for Codex1 changes in a way that materially alters the destination or autonomy promise.
- Review or qualification reveals a recurring architectural failure theme that cannot be honestly resolved within the current locked destination framing.
- The skills-first, native-Codex, or umbrella-mission constraints need to change materially.

## Provenance

### User-Stated Intent

- Codex1 should be the perfect way for Codex to work: clarify until clear, plan perfectly, then execute autonomously until the goal is achieved.
- The umbrella mission is complete only when five review sub-agents reviewing against the PRD find no agreed `P0`, `P1`, or `P2` issues.

### Repo-Grounded Facts

- The current repo already has native skills plus deterministic internal backend commands rather than a wrapper-runtime product.
- Current contract truth is still distributed enough that repeated review can keep finding new high-integrity issues in new corners.

### Codex Clarifying Synthesis

- The umbrella mission should optimize for making recurring review findings converge into central contracts rather than proliferating across many surfaces.
- Planning now owns the job of turning this destination contract into the strongest practical architecture, decomposition, proof design, and Ralph execution route for the repo.
