---
artifact: program-blueprint
mission_id: contract-centered-architecture
version: 1
lock_revision: 1
blueprint_revision: 10
plan_level: 5
risk_floor: 5
problem_size: XL
status: approved
proof_matrix:
- claim_ref: claim:clarify_intent_lock
  statement: Clarify can turn vague asks into an explicit durable outcome lock without relying on hidden chat memory.
  required_evidence:
  - PLANS/contract-centered-architecture/OUTCOME-LOCK.md
  - .codex/skills/clarify/SKILL.md
  - docs/runtime-backend.md
  review_lenses:
  - spec_conformance
  - correctness
  - evidence_adequacy
  governing_contract_refs:
  - lock:1
  - skill:clarify
  - gate:outcome_lock
- claim_ref: claim:execution_ralph_autonomy
  statement: Execute and autopilot can continue under Ralph truth, honor the approved autonomy boundary, and avoid false terminality.
  required_evidence:
  - .ralph/missions/contract-centered-architecture/closeouts.ndjson
  - .ralph/missions/contract-centered-architecture/state.json
  - docs/qualification/README.md
  review_lenses:
  - correctness
  - safety_security_policy
  - operability_rollback_observability
  governing_contract_refs:
  - ralph
  - execution_package
  - autopilot
- claim_ref: claim:planning_no_invention
  statement: Planning can produce blueprint, spec, and package truth strong enough that execution does not invent major architecture or proof contracts.
  required_evidence:
  - PLANS/contract-centered-architecture/PROGRAM-BLUEPRINT.md
  - PLANS/contract-centered-architecture/specs/
  - .ralph/missions/contract-centered-architecture/execution-packages/
  review_lenses:
  - spec_conformance
  - correctness
  - evidence_adequacy
  governing_contract_refs:
  - blueprint
  - execution_package
- claim_ref: claim:qualification_evidence_honesty
  statement: Supported-build qualification judges raw evidence rather than optimistic summaries and can distinguish surface gaps from product bugs.
  required_evidence:
  - docs/qualification/README.md
  - docs/qualification/native-multi-agent-resume-note.md
  - .codex1/qualification/reports/
  review_lenses:
  - evidence_adequacy
  - correctness
  - operability_rollback_observability
  governing_contract_refs:
  - qualification
  - supported_build
- claim_ref: claim:review_clean_closeout
  statement: Mission close requires integrated review evidence strong enough that five PRD-based review lanes surface no agreed P0, P1, or P2 findings.
  required_evidence:
  - PLANS/contract-centered-architecture/REVIEW-LEDGER.md
  - docs/codex1-prd.md
  - .ralph/missions/contract-centered-architecture/bundles/
  review_lenses:
  - spec_conformance
  - correctness
  - evidence_adequacy
  governing_contract_refs:
  - review
  - mission_close
- claim_ref: claim:support_surface_reversible
  statement: Setup, doctor, restore, and uninstall mutate the support surface reversibly and honestly.
  required_evidence:
  - docs/qualification/gates.md
  - .codex1/qualification/latest.json
  - crates/codex1-core/src/backup.rs
  review_lenses:
  - correctness
  - operability_rollback_observability
  - evidence_adequacy
  governing_contract_refs:
  - support_surface
  - qualification
decision_obligations:
- obligation_id: obligation:artifact_contract_source
  question: Where should visible artifact requirements live so validators, templates, and docs stop drifting?
  why_it_matters: Marker-based and prose-only artifact checks keep allowing structurally weak artifacts to pass and drift out of parity.
  affects:
  - architecture_boundary
  - proof_design
  - review_contract
  - protected_surface_risk
  governing_contract_refs:
  - artifacts
  - templates
  review_contract_refs:
  - review:spec
  - review:mission_close
  mission_close_claim_refs:
  - claim:clarify_intent_lock
  - claim:planning_no_invention
  - claim:review_clean_closeout
  blockingness: major
  candidate_route_count: 2
  required_evidence:
  - crates/codex1/src/internal/mod.rs
  - crates/codex1-core/src/artifacts.rs
  - templates/mission
  status: selected
  resolution_rationale: Create a machine-readable artifact-requirements registry and drive validators and scaffolds from that one source.
  evidence_refs:
  - crates/codex1/src/internal/mod.rs
  - crates/codex1-core/src/artifacts.rs
  - templates/mission
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
- obligation_id: obligation:execution_authority_contract
  question: How should fully autonomous execute and autopilot authority be made strong without losing parity or honesty?
  why_it_matters: The user-approved product promise includes irreversible autonomous execution after clarify and planning, which raises the proof and governance burden materially.
  affects:
  - migration_rollout
  - proof_design
  - review_contract
  - execution_sequencing
  - protected_surface_risk
  governing_contract_refs:
  - skill:execute
  - skill:autopilot
  - ralph
  - qualification
  review_contract_refs:
  - review:spec
  - review:mission_close
  mission_close_claim_refs:
  - claim:execution_ralph_autonomy
  - claim:qualification_evidence_honesty
  - claim:review_clean_closeout
  blockingness: critical
  candidate_route_count: 2
  required_evidence:
  - .codex/skills/execute/SKILL.md
  - .codex/skills/autopilot/SKILL.md
  - docs/codex1-prd.md
  - docs/qualification/README.md
  status: selected
  resolution_rationale: Keep planning bounded, make execution and autopilot fully autonomous under Ralph and package truth, and prove that authority through stronger runtime and qualification contracts rather than wrapper glue.
  evidence_refs:
  - .codex/skills/execute/SKILL.md
  - .codex/skills/autopilot/SKILL.md
  - docs/codex1-prd.md
  - docs/qualification/README.md
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
- obligation_id: obligation:qualification_evidence_contract
  question: How should Codex1 prove live supported-build behavior honestly enough for its product claims?
  why_it_matters: If live qualification still judges summaries instead of raw evidence, the umbrella proof bar remains soft at the exact place it needs to be strongest.
  affects:
  - proof_design
  - review_contract
  - blast_radius
  governing_contract_refs:
  - qualification
  - supported_build
  review_contract_refs:
  - review:mission_close
  mission_close_claim_refs:
  - claim:execution_ralph_autonomy
  - claim:qualification_evidence_honesty
  - claim:review_clean_closeout
  blockingness: critical
  candidate_route_count: 2
  required_evidence:
  - docs/qualification/README.md
  - docs/qualification/native-multi-agent-resume-note.md
  - crates/codex1/src/commands/qualify.rs
  status: selected
  resolution_rationale: Split qualification into raw evidence capture plus deterministic assessment, and preserve the exact artifacts used for judgment.
  evidence_refs:
  - docs/qualification/README.md
  - docs/qualification/native-multi-agent-resume-note.md
  - crates/codex1/src/commands/qualify.rs
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
- obligation_id: obligation:state_center_of_gravity
  question: What should be the single center of gravity for mission truth and legal transitions?
  why_it_matters: It determines whether recurring contract-integrity failures converge into one kernel or keep reappearing across many surfaces.
  affects:
  - architecture_boundary
  - proof_design
  - review_contract
  - execution_sequencing
  - blast_radius
  - protected_surface_risk
  governing_contract_refs:
  - lock:1
  - ralph
  - artifacts
  review_contract_refs:
  - review:spec
  - review:mission_close
  mission_close_claim_refs:
  - claim:planning_no_invention
  - claim:execution_ralph_autonomy
  - claim:review_clean_closeout
  blockingness: critical
  candidate_route_count: 3
  required_evidence:
  - docs/codex1-prd.md
  - crates/codex1-core/src/ralph.rs
  - crates/codex1-core/src/runtime.rs
  - crates/codex1/src/internal/mod.rs
  status: selected
  resolution_rationale: Adopt a native mission-contract kernel plus projection adapters, and reject both distributed patch-hardening and wrapper-runtime ownership.
  evidence_refs:
  - docs/codex1-prd.md
  - crates/codex1-core/src/ralph.rs
  - crates/codex1-core/src/runtime.rs
  - crates/codex1/src/internal/mod.rs
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
- obligation_id: obligation:support_surface_mutation_protocol
  question: How should support-surface mutation become reversible and honest enough for the product claim?
  why_it_matters: Crash windows and duplicated manifest logic undermine the same trust story the mission needs for setup, restore, uninstall, and doctor.
  affects:
  - migration_rollout
  - rollback_viability
  - blast_radius
  governing_contract_refs:
  - support_surface
  - qualification
  review_contract_refs: []
  mission_close_claim_refs:
  - claim:support_surface_reversible
  - claim:review_clean_closeout
  blockingness: major
  candidate_route_count: 2
  required_evidence:
  - crates/codex1-core/src/backup.rs
  - crates/codex1/src/commands/setup.rs
  - crates/codex1/src/commands/restore.rs
  - crates/codex1/src/commands/uninstall.rs
  status: selected
  resolution_rationale: Use one shared journaled transaction engine and make the helper commands thin clients over it.
  evidence_refs:
  - crates/codex1-core/src/backup.rs
  - crates/codex1/src/commands/setup.rs
  - crates/codex1/src/commands/restore.rs
  - crates/codex1/src/commands/uninstall.rs
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
- obligation_id: obligation:workflow_surface_composition
  question: How should the public workflows compose over the stronger contracts without becoming thin veneers or separate hidden runtimes?
  why_it_matters: The locked product claim depends on better clarify UX, state-of-the-art planning, and honest autopilot parity while preserving skills as the product surface.
  affects:
  - architecture_boundary
  - review_contract
  - execution_sequencing
  - protected_surface_risk
  governing_contract_refs:
  - skill:clarify
  - skill:plan
  - skill:review
  - skill:autopilot
  - runtime_backend
  review_contract_refs:
  - review:spec
  - review:mission_close
  mission_close_claim_refs:
  - claim:clarify_intent_lock
  - claim:planning_no_invention
  - claim:review_clean_closeout
  blockingness: critical
  candidate_route_count: 2
  required_evidence:
  - .codex/skills/clarify/SKILL.md
  - .codex/skills/plan/SKILL.md
  - .codex/skills/review/SKILL.md
  - .codex/skills/autopilot/SKILL.md
  - /Users/joel/oh-my-codex/skills/deep-interview/SKILL.md
  - /Users/joel/oh-my-codex/skills/ralplan/SKILL.md
  status: selected
  resolution_rationale: Keep public workflows explicit in skills, strengthen them using the new contract stack, and borrow only the best OMX questioning and critique ideas.
  evidence_refs:
  - .codex/skills/clarify/SKILL.md
  - .codex/skills/plan/SKILL.md
  - .codex/skills/review/SKILL.md
  - .codex/skills/autopilot/SKILL.md
  - /Users/joel/oh-my-codex/skills/deep-interview/SKILL.md
  - /Users/joel/oh-my-codex/skills/ralplan/SKILL.md
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
selected_target_ref: mission:contract-centered-architecture
---
# Program Blueprint

## Locked Mission Reference

- Mission id: `contract-centered-architecture`
- Lock revision: `1`
- Mission class: umbrella product mission
- Selected planning rigor: `level 5`
- Problem size: `XL`
- Current selected target: `mission:contract-centered-architecture`

## Truth Register Summary

- Verified repo truth: Codex1 already has a native skills surface plus deterministic internal commands, but mission legality and product-critical invariants are still spread across visible artifacts, hidden Ralph state, validators, and qualification helpers.
- Verified risk: recurring `P0`/`P1`/`P2`-grade findings are not just a finite bug list; they are repeated manifestations of distributed contract truth and underconstrained state transitions.
- Locked user intent: `clarify` must interview until the mission is genuinely clear, `plan` must produce the strongest practical spec-driven plans, and `execute` plus `autopilot` must continue autonomously until the goal is truly done.
- Locked proof bar: the umbrella mission is not complete until five review sub-agents reviewing against the PRD surface no agreed `P0`, `P1`, or `P2` findings, and recurring findings trigger rethink or replan rather than endless edge-patching.
- OMX inspiration to preserve: intent-first deep interview pressure, explicit planning critique loops, and strong validation discipline.
- OMX traits to reject: wrapper-runtime ownership, tmux or team orchestration as the product core, and hidden state or control planes that outrank repo artifacts.

## System Model

- Public product surface: `.codex/skills/{clarify,plan,execute,review,autopilot}`.
- Deterministic machine surface: `codex1 internal init-mission`, `materialize-plan`, `compile-execution-package`, `derive-writer-packet`, `compile-review-bundle`, `record-review-outcome`, `record-contradiction`, `resolve-resume`, and validators.
- Visible mission truth: `PLANS/<mission-id>/README.md`, `MISSION-STATE.md`, `OUTCOME-LOCK.md`, `PROGRAM-BLUEPRINT.md`, `specs/*/SPEC.md`, `REVIEW-LEDGER.md`, `REPLAN-LOG.md`.
- Hidden machine truth: `.ralph/missions/<mission-id>/{state.json,active-cycle.json,closeouts.ndjson,gates.json,execution-packages,packets,bundles,contradictions.ndjson,execution-graph.json}`.
- Support-surface truth: `.codex/config.toml`, `.codex/hooks.json`, `AGENTS.md`, copied or linked skills, backup manifests, and qualification evidence.
- Supported environment truth: native Codex hooks, native `multi_agent_v2`, trusted-build qualification, and Ralph continuity through native stop and resume surfaces.

## Invariants And Protected Behaviors

- Skills remain the product surface; helper commands remain subordinate persistence and inspection helpers.
- Mission truth must converge toward one authoritative native derivation path rather than remain split across loosely coupled neighboring records.
- `state.json`, `gates.json`, and README summaries must become projections or caches of stronger canonical truth, not peer authorities.
- Execution may not begin without a passed execution package, and review may not be bypassed on the way to completion.
- Non-terminal verdicts must never surface as terminal completion.
- Manual public-skill progression and `autopilot` must converge to the same artifact truth and gate outcomes.
- Native Codex behavior must remain primary; wrapper-runtime ownership must not be reintroduced under a different name.
- Fully autonomous execution authority after clarify and planning must still remain subordinate to the locked mission contract and Ralph truth.

## Proof Matrix

- `claim:clarify_intent_lock` — Clarify can turn a vague ask into an explicit, bounded, durable outcome lock without relying on hidden chat memory.
- `claim:planning_no_invention` — Planning can produce a blueprint, frontier specs, and a passed package strong enough that execution does not invent major architecture or proof contracts.
- `claim:execution_ralph_autonomy` — Execute and autopilot can continue under Ralph truth, honor the approved autonomy boundary, and avoid false terminality.
- `claim:support_surface_reversible` — Setup, doctor, restore, and uninstall act transactionally and honestly enough that managed support surfaces remain reversible and diagnosable.
- `claim:qualification_evidence_honesty` — Qualification proves the real supported build behavior from raw evidence rather than optimistic summaries.
- `claim:review_clean_closeout` — Mission close requires integrated review evidence strong enough that five PRD-based review lanes surface no agreed `P0`, `P1`, or `P2` findings.

## Decision Obligations

- `obligation:state_center_of_gravity` — selected native mission-contract kernel plus projections over distributed patch-hardening or wrapper ownership.
- `obligation:support_surface_mutation_protocol` — selected journaled transaction engine over imperative manifest-as-mutation-protocol scripts.
- `obligation:artifact_contract_source` — selected machine-readable artifact requirements registry over hand-maintained marker checks and prose-only duplication.
- `obligation:qualification_evidence_contract` — selected raw evidence capture plus deterministic judge split over model-summary-first proof.
- `obligation:workflow_surface_composition` — selected thin public skills over one shared contract stack rather than separate autopilot or runtime semantics.
- `obligation:execution_authority_contract` — selected bounded planning authority plus fully autonomous execution or autopilot authority once the mission is clear and planned.

## In-Scope Work Inventory

| Work item | Class | Why it exists | Finish in this mission? |
| --- | --- | --- | --- |
| `mission_contract_kernel` | runnable_frontier | Centralize mission legality, projections, and resume truth so recurring state-integrity findings collapse into one kernel. | yes |
| `artifact_contract_registry` | near_frontier | Replace marker-based or prose-only artifact validation with one requirements registry that can drive validators and scaffolds. | yes |
| `support_surface_txn` | near_frontier | Make setup, restore, uninstall, and doctor operate through a shared journaled transaction engine. | yes |
| `qualification_evidence_pipeline` | near_frontier | Make qualification evidence-first and raw-artifact judged so live claims are proven honestly. | yes |
| `workflow_surface_clarify_plan_review` | completed_frontier | Refit the public skills and operator-facing workflow around the stronger contracts and better clarify or planning UX. | yes |
| `execute_autopilot_governance` | runnable_frontier | Make execute and autopilot preserve manual parity, Ralph truth, and fully autonomous execution authority without hidden wrapper semantics. | yes |

## Option Set

- **Option A — patch the current distributed truth model in place**: keep `RalphState`, `CloseoutRecord`, `ActiveCycleState`, validators, and support-surface flows as separate but better checked surfaces.
- **Option B — rebuild around a wrapper runtime or OMX-style control plane**: move more orchestration into a runtime or team wrapper and keep the repo artifacts as a downstream projection.
- **Option C — native contract-centered consolidation**: keep skills and native Codex semantics, but introduce a central mission-contract kernel plus contract registries, transactional support-surface engine, evidence-first qualification, and workflow refits.

## Selected Architecture

Choose **Option C: native contract-centered consolidation**.

The route is a staged consolidation around five cores:

1. **Mission-contract kernel in `codex1-core`**
   - Introduce one normalized center of gravity for mission identity, legal state transitions, and derived mission snapshots.
   - Treat `state.json`, `gates.json`, and README-level summaries as projections of stronger canonical truth.
   - Make resume, closeout, and validator logic consume the same kernel instead of reconstructing legality in many neighboring places.

2. **Artifact-requirements registry**
   - Define, in code, what each visible artifact must contain, when it is required, and how it binds to machine truth.
   - Generate or drive validators, templates, and parity checks from that one registry instead of scattering marker checks, prose conventions, and ad hoc parsing.

3. **Support-surface transaction engine**
   - Promote the existing reversible backup concepts into one shared transaction engine with explicit staging, commit, rollback, and crash-recovery semantics.
   - Make setup, restore, uninstall, and doctor different views over the same durable mutation journal rather than sibling reimplementations of manifest logic.

4. **Evidence-first qualification**
   - Split live qualification into raw evidence capture and deterministic judgment.
   - Persist the actual native outputs needed for stop-hook, resume, and child-lane proofs, then assess those artifacts directly.

5. **Workflow surface refit**
   - Refit `clarify`, `plan`, `review`, `execute`, and `autopilot` to sit on the same contract stack.
   - Preserve the strong OMX ideas we want: intent-first interview pressure, critique-driven planning, and evidence-backed validation.
   - Explicitly reject OMX’s wrapper or team runtime ownership as the product substrate.

The core consolidation slices plus the public workflow refit are now review-clean. The current selected target remains the mission frontier `mission:contract-centered-architecture`, packaging those completed slices together with the newly runnable `execute_autopilot_governance` slice so the final autonomy-governance refit stays dependency-complete and honest.

## Rejected Alternatives and Rationale

- **Reject Option A (distributed patch-hardening)** because the recurring review pattern is structural: the same invariant can still fail in new corners when truth is split across closeouts, active-cycle state, cached state, gate files, contradiction files, validation heuristics, and support-surface scripts.
- **Reject Option B (wrapper-runtime ownership)** because the PRD explicitly forbids rebuilding OMX in a cleaner shape. Skills, visible artifacts, and native Codex seams must remain the product surface.
- **Reject a big-bang rewrite** because the repo already contains useful contracts, tests, and deterministic commands. The route should salvage those semantics while centralizing their authority.

## Migration / Rollout / Rollback Posture

- Stage the work behind the current deterministic internal command surface so public skills can improve without inventing a second runtime.
- Preserve on-disk mission artifact classes and helper command entrypoints while moving their authority toward the central kernel and registries.
- Use compatibility-preserving projections while each core is introduced, then remove duplicated logic only after the new path is validated.
- Re-run internal validation, execution-package checks, qualification gates, and review waves after each major slice.
- Child missions or subordinate specs may be used for execution convenience, but they must remain subordinate to this umbrella destination contract.

## Review Bundle Design

- Mandatory review lenses: spec_conformance, correctness, interface_compatibility, safety_security_policy, evidence_adequacy, operability_rollback_observability.
- Every spec review must bind the reviewer to the exact governing package, changed contract surfaces, proof receipts, and touched files or diffs.
- Mission-close review must include the lock, blueprint invariants, cross-spec claims, qualification evidence relevant to the changed seams, and the integrated review history.
- The final mission-close bar is five independent PRD-based review lanes with no agreed `P0`, `P1`, or `P2` findings.
- Reviewer disagreement alone does not block completion; only findings the user agrees with count toward the umbrella proof bar.

## Workstream Overview

- `mission_contract_kernel` establishes the central native authority for mission legality and projections.
- `artifact_contract_registry` makes visible-artifact truth machine-readable and generator-driven.
- `support_surface_txn` makes helper-surface mutation transactional and crash-recoverable.
- `qualification_evidence_pipeline` makes live supported-build claims evidence-first and auditable.
- `workflow_surface_clarify_plan_review` strengthens the public skill UX and keeps it aligned to repo truth.
- `execute_autopilot_governance` makes fully autonomous execution and autopilot honest, parity-safe, and Ralph-governed.

## Execution Graph and Safe-Wave Rules

- The current runnable frontier now advances from the completed kernel, artifact, support-surface, qualification, and workflow slices into `execute_autopilot_governance`, which is the final remaining implementation slice before mission-close proof.
- No same-workspace parallel write wave is authorized until the kernel slice lands and later slices are re-evaluated against explicit graph safety.
- Read-only planning scouts and review helpers may fan out broadly up to the supported native breadth, but write execution remains singleton by default.
- Promotion order after the completed core and workflow slices is now: `execute_autopilot_governance`, then mission-close proof and the five-reviewer PRD cleanliness bar once the autonomy-governance refit is review-clean.

## Risks And Unknowns

- Fully autonomous execution authority raises the proof burden for Ralph and qualification; the product claim is stronger, so the runtime and review surfaces must be proportionally stronger.
- The route must avoid overfitting to the current review findings while still centralizing the recurring failure themes they reveal.
- The artifact registry and kernel split must improve clarity rather than creating another abstraction layer that hides simple truths.
- The public skill surface must become stronger without becoming a thin facade over hidden workflow code.

## Decision Log

| Decision id | Statement | Rationale | Evidence refs | Adopted in revision |
| --- | --- | --- | --- | --- |
| D1 | Make a native mission-contract kernel the first runnable slice. | The recurring review pattern points to distributed authority and underconstrained legality as the highest-leverage root cause. | `docs/codex1-prd.md`, `crates/codex1-core/src/ralph.rs`, `crates/codex1-core/src/runtime.rs`, `crates/codex1/src/internal/mod.rs` | 1 |
| D2 | Keep public skills as the product surface and deterministic internal commands as persistence helpers. | The PRD and lock both require skills-first UX and reject helper-CLI-first or wrapper-runtime-first ownership. | `docs/codex1-prd.md`, `docs/runtime-backend.md`, `PLANS/contract-centered-architecture/OUTCOME-LOCK.md` | 1 |
| D3 | Take inspiration from OMX deep-interview pressure and critique loops, but not from OMX wrapper or team runtime ownership. | The user explicitly wants the best parts of OMX question quality and planning discipline without inheriting its hackier orchestration substrate. | `/Users/joel/oh-my-codex/skills/deep-interview/SKILL.md`, `/Users/joel/oh-my-codex/skills/ralplan/SKILL.md`, `/Users/joel/oh-my-codex/docs/qa/deep-interview-phase-1-validation.md` | 1 |
| D4 | Make support-surface mutation transactional and qualification evidence-first as first-class subsystems, not cleanup tasks. | Helper-surface crash windows and proxy-proof qualification currently undermine the same trust story the product is trying to claim. | `crates/codex1/src/commands/setup.rs`, `crates/codex1/src/commands/restore.rs`, `crates/codex1/src/commands/uninstall.rs`, `crates/codex1/src/commands/qualify.rs`, `docs/qualification/gates.md` | 1 |
| D5 | Treat final review cleanliness as a route-shaping contract, not a post-hoc QA wish. | The user’s done bar is explicit: five PRD-based reviewers with no agreed `P0`, `P1`, or `P2` findings. | `PLANS/contract-centered-architecture/OUTCOME-LOCK.md`, `docs/codex1-prd.md` | 1 |

## Replan Policy

- Reopen the **Outcome Lock** if the desired product claim, skills-first stance, umbrella-mission framing, or full autonomous execution promise changes materially.
- Reopen the **Blueprint** if the contract-centered route fails critique, recurring findings reveal a different deeper center of gravity, or manual and autopilot parity needs a different architectural shape.
- Reopen at **execution_package** if the first runnable slice changes scope, dependency truth, or proof or review obligations while the overall route still stands.
- Preserve salvage aggressively: valid specs, proofs, and review evidence stay live unless the higher layer explicitly invalidates them.
