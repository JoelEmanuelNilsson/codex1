---
artifact: mission-state
mission_id: contract-centered-architecture
root_mission_id: contract-centered-architecture
parent_mission_id: null
version: 1
clarify_status: ratified
slug: contract-centered-architecture
current_lock_revision: 1
reopened_from_lock_revision: null
---
# Mission State

## Objective Snapshot

- Mission title: Carry Codex1 to its full reliable end state as the umbrella mission
- Current interpreted objective: Codex1 should become the best way for Codex to work: `clarify` destroys ambiguity, `plan` produces state-of-the-art spec-driven plans, `execute` and `autopilot` continue under Ralph until the user goal is truly done, and the repo becomes essentially `P0`/`P1`/`P2`-clean against the PRD.
- Current phase hint: planning

## Ambiguity Register

| Dimension | Score (0-3) | Why it still matters | Planned reducer | Provenance |
| --- | --- | --- | --- | --- |
| Objective clarity | 0 | The umbrella mission destination is now explicit and ratified. | None; objective is locked. | User-stated and ratified. |
| Success proof | 1 | The high-level proof bar is explicit, and planning now needs to turn it into concrete proof and review design. | Convert the proof bar into planning proof contracts and review gates. | User-stated and Codex synthesis. |
| Protected surfaces | 0 | The required product constraints are explicit enough for planning. | None; protected surfaces are lock-ready. | User-stated, repo-grounded, and ratified. |
| Tradeoff vetoes | 0 | The unacceptable tradeoffs are explicit enough for planning. | None; tradeoff vetoes are lock-ready. | User-stated and ratified. |
| Scope boundary | 0 | This is the umbrella mission, with later child missions allowed only as subordinate execution structure. | None; scope is locked. | User-stated and ratified. |
| Autonomy boundary | 1 | The major autonomy split is explicit: planning stays bounded, while execution and autopilot may act fully autonomously once the mission is clear and planned. Planning must still encode this precisely. | Turn the autonomy contract into explicit planning and Ralph policy. | User-stated and Codex synthesis. |
| Baseline facts | 0 | Repo baseline facts are strong enough for planning to begin. | None; baseline facts are sufficient. | Repo-grounded. |
| Rollout or migration constraints | 1 | The umbrella mission owns the full end state, and planning now needs to decide the cleanest execution structure under that umbrella. | Resolve route and child-mission posture during planning. | User-stated and Codex synthesis. |

## Candidate Success Criteria

- A user can start with `clarify`, answer excellent high-leverage questions, and reach a genuinely trustworthy lock.
- `plan` can produce the strongest practical spec-driven plan shape, including umbrella planning plus clear bounded specs for execution.
- `execute` and `autopilot` can continue autonomously according to the approved authority contract until the mission is actually done, reviewed, and ready.

## Protected Surface Hypotheses

- Keep the public skills surface and do not remove existing skills.
- Improve skill quality aggressively, including `clarify`, if needed.
- Stay native to Codex and less hacky, less over-engineered, and more reliable than OhMyCodex-style inspiration.

## Baseline Repo Facts

| Fact | Provenance | Evidence ref | Confidence |
| --- | --- | --- | --- |
| The repo currently has deterministic internal commands for mission bootstrap, planning, execution packages, review bundles, contradictions, resume, and validation. | Repo read | `/Users/joel/codex1/docs/runtime-backend.md`. | high |
| Current mission truth and machine legality are still distributed across visible artifacts plus hidden Ralph state such as `state.json`, `active-cycle.json`, `closeouts.ndjson`, and `gates.json`. | Repo read | `/Users/joel/codex1/crates/codex1-core/src/paths.rs`, `/Users/joel/codex1/crates/codex1-core/src/ralph.rs`. | high |
| The mission is now the active umbrella clarify root for the full Codex1 end state and is ready to hand off to planning. | Runtime writeback | `/Users/joel/codex1/PLANS/contract-centered-architecture` and `/Users/joel/codex1/.ralph/missions/contract-centered-architecture`. | high |

## Open Assumptions

- `Perfect` means the strongest practical Codex1 workflow and planning product we can honestly ship in this repo, not a claim of formal optimality.
- The five-reviewer cleanliness bar is judged by whether you agree with any `P0`/`P1`/`P2` findings they surface, not by raw sub-agent output alone.
- AGENTS.md-related complaints that you judge inconsistent with stronger repo truth do not by themselves block completion.

## Highest-Value Next Question

Clarify is complete. The next step is `$plan`, not another clarify question.

## Feasibility Notes

- Probe used: Repeatedly compare the repo’s current structure and product constraints against the user’s desired Codex1 end state.
- Result: The mission is now lock-ready and clarify no longer needs additional user answers before planning.
- Constraint surfaced: Planning must now centralize the recurring contract-integrity failure modes without weakening the native Codex, skills-first product stance.
