---
artifact: mission-state
mission_id: ralph-control-loop-boundary
root_mission_id: ralph-control-loop-boundary
parent_mission_id: null
version: 1
clarify_status: ratified
slug: ralph-control-loop-boundary
current_lock_revision: 1
reopened_from_lock_revision: null
---

# Mission State

## Objective Snapshot

- Mission title: Ralph Control Loop Boundary
- Current interpreted objective: Make Ralph continuation apply only to explicit parent/orchestrator loop modes, while normal user conversation and all subagents can stop without being blocked by mission gates.
- Current phase hint: planning

## Ambiguity Register

| Dimension | Score (0-3) | Why it still matters | Planned reducer | Provenance |
| --- | --- | --- | --- | --- |
| Objective clarity | 0 | The user clearly wants Ralph scoped to explicit parent loop workflows. | Locked in `OUTCOME-LOCK.md`. | user ask |
| Success proof | 1 | Exact test names are for planning, but observable behavior is clear. | Plan proof rows for user-turn, subagent, and parent-loop stop-hook probes. | user + repo |
| Protected surfaces | 1 | Exact files may expand during planning, but the sensitive surfaces are known. | Plan scoped implementation around stop-hook/control-plane surfaces. | repo |
| Tradeoff vetoes | 0 | The user explicitly rejects global blocking, subagent blocking, and hook-file hand editing as normal UX. | Locked in `OUTCOME-LOCK.md`. | user ask |
| Scope boundary | 1 | Planning must choose the smallest route that fixes the control loop without redesigning everything. | `$plan` should decompose into bounded control-plane specs. | clarify synthesis |
| Autonomy boundary | 1 | Codex may choose implementation shape, but not remove skills or weaken autopilot. | Locked autonomy boundary. | user ask |
| Baseline facts | 0 | Repo reading confirmed the current narrow child-review bypass and global resume path. | Use refs below. | repo |
| Rollout or migration constraints | 1 | Setup/restore implications should be planned, not guessed during clarify. | Include setup/qualification checks in plan. | repo + inference |

## Candidate Success Criteria

- User-interaction stop-hook turns yield even when a mission has an open or failed gate.
- Generic subagent stop-hook turns yield even when a mission has an open or failed gate.
- Explicit parent loop turns still block on open/failed gates when `$plan`, `$execute`, `$review-loop`, or `$autopilot` owns the continuation lease.
- A first-class pause/close escape exists so users do not need to move `.codex/hooks.json`.

## Protected Surface Hypotheses

- `crates/codex1/src/internal/mod.rs` stop-hook input parsing and bypass logic.
- `crates/codex1-core/src/runtime.rs` `resolve_stop_hook_output` and resume-to-stop decision logic.
- `.codex/skills/{plan,execute,review-loop,autopilot,clarify}` loop entry/exit language.
- Setup/qualification docs and tests that prove the hook remains installed but sane.

## Baseline Repo Facts

| Fact | Provenance | Evidence ref | Confidence |
| --- | --- | --- | --- |
| Stop hook currently special-cases findings-only reviewer lanes before normal Ralph resume handling. | repo | `crates/codex1/src/internal/mod.rs:555` | high |
| Normal stop-hook handling routes through `resolve_resume`. | repo | `crates/codex1-core/src/runtime.rs:5131` | high |
| The current bypass is reviewer-specific, not a general subagent exemption or user-interaction boundary. | repo + inference | `crates/codex1/src/internal/mod.rs:576` | high |
| The emergency manual workaround is moving/disabling `.codex/hooks.json`. | user + repo | `.codex/hooks.json` | high |

## Open Assumptions

- The exact control-plane artifact name can be chosen during planning.
- The exact public escape skill name can be chosen during planning.
- Native stop-hook input has enough signal, or can be combined with durable local lease state, to distinguish parent loop enforcement from ordinary conversation.

## Highest-Value Next Question

No user-owned blocker remains for planning. Start `$plan` for the `ralph-control-loop-boundary` mission.

## Feasibility Notes

- Probe used: read current stop-hook entry and core stop-hook resolver.
- Result: feasible control boundary exists because the CLI already branches on stop-hook input before core resume handling.
- Constraint surfaced: current code only handles narrow reviewer-lane metadata; broader user-interaction and all-subagent semantics need a durable control-plane contract.
