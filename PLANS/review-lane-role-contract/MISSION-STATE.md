---
artifact: mission-state
mission_id: review-lane-role-contract
root_mission_id: review-lane-role-contract
parent_mission_id: null
version: 1
clarify_status: ratified
slug: review-lane-role-contract
current_lock_revision: 1
reopened_from_lock_revision: null
---

# Mission State

## Objective Snapshot

- Mission title: Review Lane Role Contract
- Current interpreted objective: Clarify and redesign Codex1 review orchestration so parent review loops, dedicated reviewer agents, model routing, and Ralph stop semantics cannot deadlock or let child reviewers mutate mission truth.
- Current phase hint: clarify

## Ambiguity Register

| Dimension | Score (0-3) | Why it still matters | Planned reducer | Provenance |
| --- | --- | --- | --- | --- |
| Objective clarity | 1 | User has named the core product shape: `$review-loop` is the parent/orchestrator loop, direct reviewers do not need a skill, and child reviewers are findings-only. | Clarify remaining routing and proof details. | user ask |
| Success proof | 1 | Clean means no P0, P1, or P2 findings in the latest loop; any P0-P2 finding means not clean. Separate default profiles are now required for local/spec, integration, and mission-close review. Code-producing work should also run bug/correctness reviewers at appropriate checkpoints. | Planning may define exact checkpoint placement. | user ask |
| Protected surfaces | 1 | Main surfaces are known and user decided the existing `$review` skill name should be removed, not kept as alias or direct-review UX. | Preserve the rename/removal contract in the lock. | repo + user |
| Tradeoff vetoes | 1 | User rejects endless review loops, wants `gpt-5.4` for spec/intent/PRD judgment, and wants bug/correctness reviewers whenever code-producing work needs review. | Preserve this model-routing intent in the lock. | user ask |
| Scope boundary | 1 | User wants `$review-loop` and findings-only reviewers; runtime/Ralph enforcement details, output schema, and exact review checkpoints are delegated to planning. | Preserve delegated planning boundaries in the lock. | user ask + repo |
| Autonomy boundary | 1 | Parent owns orchestration and review writeback; child reviewers return findings only and do not run Ralph loops. | Clarify exact child allowed actions. | user ask |
| Baseline facts | 1 | Repo facts show the current review skill mixes parent orchestration, reviewer judgment, and writeback responsibilities; internal orchestration already says parent owns mission truth. | Keep reading only if a later answer depends on exact implementation. | repo |
| Rollout or migration constraints | 1 | Compatibility with current `$review` skill name is not required; user wants the review-name skill removed. | Planning may choose safe migration details without preserving `$review` as a skill. | user ask |

## Candidate Success Criteria

- There is a clear product distinction between parent review orchestration and child reviewer execution.
- Child review lanes cannot deadlock because parent Ralph gates are open, and cannot clear or mutate mission truth.
- The review workflow supports multiple review types with intentional model routing, such as cheap code-bug review, integrated intent review, and final mission-close review.
- `$review-loop` runs review/fix/review cycles until no findings remain or six consecutive review loops have run; if findings persist after six loops, the parent routes to replan instead of continuing indefinitely.
- Direct single-review agents are prompt-only findings roles by default and do not need a skill.
- Default local/spec review should not include bug/correctness reviewer agents.
- Default local/spec review should use `gpt-5.4` spec and intent judgment reviewers.
- Default PRD/intent and mission-close judgment can use two `gpt-5.4` reviewers.
- Code-producing work should run `gpt-5.3-codex` bug/correctness reviewers at
  appropriate review checkpoints.
- Any P0, P1, or P2 finding means the loop is not clean; P3 findings do not block cleanliness by default.
- `$review-loop` should define separate default profiles for local/spec review
  after one execution slice, integration review after multiple slices or
  phases, and final mission-close review before completion.
- Each of those three default profiles should include a dedicated `gpt-5.4`
  judgment lane.
- The existing `$review` skill name should be removed rather than kept as an
  alias or repurposed as direct-review UX.
- `$review-loop` should run reviews at proof-worthy boundaries, not after every
  tiny edit: after code-producing execution slices, after spec/phase
  completion, after multiple related slices for integration review, before
  mission close, and after repairs using only the relevant review profile.

## Protected Surface Hypotheses

- Public skills surface: `$review`, `$execute`, `$autopilot`, and `internal-orchestration`.
- Ralph stop-hook semantics and mission gate writeback.
- Native subagent behavior, prompts, roles, model routing, and allowed tool/write surface.

## Baseline Repo Facts

| Fact | Provenance | Evidence ref | Confidence |
| --- | --- | --- | --- |
| Current `$review` says it should use a fresh read-only reviewer context but also says to record review outcomes and update ledgers. | repo-grounded | `.codex/skills/review/SKILL.md` | high |
| Current `internal-orchestration` says the parent thread owns mission truth, final synthesis, completion judgment, artifact writeback, and reconciliation. | repo-grounded | `.codex/skills/internal-orchestration/SKILL.md` | high |
| Current config already has a dedicated `codex1_review` model lane using `gpt-5.4-mini` with high reasoning. | repo-grounded | `.codex/config.toml` | high |
| PRD says blocking review must be performed by a fresh read-only reviewer thread or reviewer role that consumes a bounded review bundle. | repo-grounded | `docs/codex1-prd.md` | high |
| User wants the orchestration skill renamed to `$review-loop` to avoid child reviewers mistakenly invoking the parent review workflow. | user-stated | current clarify answer | high |
| User wants review loops capped at six consecutive loops; persistent findings after that indicate a likely architecture/contract problem and should route to replan. | user-stated | current clarify answer | high |
| User does not want child reviewers to be prompted with Ralph keep-going behavior; they should be findings-only roles. | user-stated | current clarify answer | high |
| User wants bug/correctness review to use `gpt-5.3-codex`, normally two agents per lane. | user-stated | current clarify answer | high |
| User wants intent and PRD review to use `gpt-5.4`, normally one agent per lane. | user-stated | current clarify answer | high |
| User is unsure whether `gpt-5.4-mini` belongs in review lanes; it should not be assumed as a default review model. | user-stated | current clarify answer | high |
| Any finding above P3 means the review loop is not clean. | user-stated | current clarify answer | high |
| User wants separate default profiles for local/spec review, integration review, and final mission-close review. | user-stated | current clarify answer | high |
| User wants the local/spec, integration, and final mission-close profiles to use `gpt-5.4` judgment lanes. | user-stated | current clarify answer | high |
| User wants the existing `$review` skill name removed. | user-stated | current clarify answer | high |
| User corrected the local/spec review default: it should not include bug/correctness agents, and should instead use GPT-5.4 spec/intent judgment reviewers. | user-stated | current clarify answer | high |
| User says PRD/intent and mission-close judgment can use two GPT-5.4 reviewers. | user-stated | current clarify answer | high |
| User expects a later planning-quality mission or redesign may make planning much more thorough and deliberate; this is related context, not yet locked as in-scope here. | user-stated | current clarify answer | medium |
| User wants bug/correctness reviewers whenever code-producing work is reviewed, at appropriate times chosen by the workflow. | user-stated | current clarify answer | high |
| User agrees review should run at proof-worthy phase boundaries rather than after every tiny edit. | user-stated | current clarify answer | high |

## Open Assumptions

- Runtime/Ralph enforcement mechanics are not user-specified; planning may choose the best implementation as long as child reviewers do not deadlock or mutate mission truth.
- Planning should think through any additional lane types beyond spec/intent and PRD/mission-close judgment, but must preserve the user-specified model preferences unless there is a strong reason to reopen.
- Current inference: planning may decide exact checkpoints for `gpt-5.3-codex` bug/correctness lanes, but code-producing work should not skip them when correctness review is appropriate.
- `gpt-5.4-mini` may be excluded from review lanes by default unless planning identifies a narrow non-blocking/support review role.
- Exact local/spec `gpt-5.4` agent count and child-finding output schema are delegated to planning.

## Highest-Value Next Question

Lock is ready. Handoff to `$plan` to design the `$review-loop` architecture, reviewer role taxonomy, model routing, six-loop cap behavior, and Ralph-safe child-lane enforcement.

## Feasibility Notes

- Probe used: read current review, internal-orchestration, multi-agent, config, and PRD surfaces.
- Result: the repo already documents parent authority, but does not yet make the child reviewer role explicit enough or enforce it as a capability boundary.
- Constraint surfaced: prompt-only reviewer discipline is probably insufficient if the mission wants to prevent deadlocks and child writeback.
