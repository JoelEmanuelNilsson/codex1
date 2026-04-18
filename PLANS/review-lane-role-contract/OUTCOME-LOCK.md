---
artifact: outcome-lock
mission_id: review-lane-role-contract
root_mission_id: review-lane-role-contract
parent_mission_id: null
version: 1
lock_revision: 1
status: locked
lock_posture: unconstrained
slug: review-lane-role-contract
---

# Outcome Lock

## Objective

Draft: Redesign Codex1 review orchestration so `$review-loop` is the
parent/orchestrator skill for review/fix/review loops, direct reviewer agents
are findings-only prompt roles rather than skill users, model routing is
intentional by review type, and Ralph stop behavior cannot deadlock child review
lanes or let child reviewers mutate mission truth.

## Done-When Criteria

- `$review-loop` exists as the parent/orchestrator workflow surface for running
  review loops until clean or until the six-loop cap is reached.
- Child reviewer agents are findings-only roles that return `NONE` or findings
  with evidence refs; they do not invoke `$review-loop`, clear gates, write
  mission artifacts, or decide completion.
- If six consecutive review loops still find blocking issues, the parent routes
  to replan instead of continuing indefinitely.
- Default local/spec review does not include bug/correctness reviewer agents.
- Default local/spec review uses `gpt-5.4` spec and intent judgment reviewers.
- PRD/intent and final mission-close judgment can use two `gpt-5.4`
  reviewers.
- Code-producing work runs `gpt-5.3-codex` bug/correctness reviewers at
  appropriate review checkpoints.
- A review loop is clean only when the latest loop has no P0, P1, or P2
  findings.
- `$review-loop` has separate default profiles for local/spec review,
  integration review, and final mission-close review.
- Local/spec review, integration review, and final mission-close review each
  include a dedicated `gpt-5.4` judgment lane.
- The existing `$review` skill name is removed; it is not retained as a
  compatibility alias and is not repurposed as direct-review UX.
- `$review-loop` runs review at proof-worthy boundaries rather than after every
  tiny edit: after code-producing execution slices, after spec/phase
  completion, after related multi-slice integration, before mission close, and
  after repairs using only the relevant review profile.

## Success Measures

- No parent review loop can wait forever on a child reviewer blocked by parent
  Ralph gates.
- No child reviewer can be the authority that records final review outcome,
  clears gates, or terminalizes a mission.
- Review outcomes are explainable by lane outputs and loop count: clean means
  no blocking findings remain in the latest loop; capped means six loops ran
  and replan is required.
- Model routing is explainable by review type: `gpt-5.4` is used for
  spec/intent, PRD, integration intent, and mission-close judgment. Any
  `gpt-5.3-codex` bug/correctness lane must be a separate code-review or
  hard-technical profile, not part of default local/spec intent judgment.
- Review profile selection is explainable by scope: local/spec review,
  integration review, and mission-close review are distinct defaults rather
  than one generic prompt.
- Review timing is explainable by proof boundary: code-producing slices,
  completed specs/phases, integration points, mission close, and targeted repair
  re-review.

## Protected Surfaces

- Public skills surface: new `$review-loop`, removal of existing `$review`,
  `$execute`, `$autopilot`, and `internal-orchestration`.
- Ralph stop-hook and mission gate semantics.
- Native subagent roles, prompts, tool permissions, model routing, and review
  writeback authority.

## Unacceptable Tradeoffs

- Do not allow endless review/fix/review cycles beyond six consecutive loops.
- Do not make child reviewers run the parent review workflow or Ralph
  keep-going loop.
- Do not rely on child reviewers to mutate mission truth or decide completion.
- Do not treat P0, P1, or P2 findings as clean.
- Do not default review judgment to `gpt-5.4-mini` unless planning identifies a
  narrow support-only role that does not weaken blocking review quality.
- Do not preserve `$review` as a skill name.

## Non-Goals

- Replacing native Codex subagents with an external wrapper or babysitter
  runtime.
- Turning direct single-review agents into a required public skill workflow.
- Making `$review-loop` a substitute for planning, execution, or mission-close
  truth; it remains a review orchestration workflow.

## Autonomy Boundary

- Codex may decide later without asking: the exact runtime/Ralph enforcement
  mechanics and the implementation details for child lane prompting and timeout
  handling, as long as the locked behavior is preserved. Codex may also design
  additional review-lane types and review profiles as long as spec/intent,
  PRD, integration, and mission-close routing preserve the model-quality intent.
  Codex may choose exact local/spec `gpt-5.4` agent counts, exact
  bug/correctness checkpoints, and the child-reviewer output schema during
  planning.
- Codex must ask before deciding: changing the six-loop cap, allowing child
  reviewers to mutate mission truth, allowing reviewers to run the parent
  review loop, replacing `$review-loop` as the parent/orchestrator surface, or
  lowering the blocking threshold so P0/P1/P2 findings can be considered clean,
  or preserving `$review` as a skill name.

## Locked Field Discipline

The fields above for objective, done-when criteria, protected surfaces,
unacceptable tradeoffs, non-goals, autonomy boundary, and reopen conditions are
locked fields. Change them only through an explicit reopen or superseding lock
revision, never by silent mutation.

## Baseline Current Facts

- Current `$review` mixes parent orchestration, reviewer judgment, and review
  writeback instructions.
- Current `internal-orchestration` already says the parent owns mission truth,
  completion judgment, artifact writeback, and reconciliation.
- Current config already has a `codex1_review` model lane using
  `gpt-5.4-mini`.
- User wants `$review-loop` as the parent/orchestrator skill name to avoid
  confusing direct reviewers with the review loop.
- User wants child reviewers to be findings-only prompt roles and not to have a
  Ralph keep-going loop.
- User wants review loops capped at six consecutive loops before replanning.
- User originally considered bug/correctness review with `gpt-5.3-codex`, but
  later clarified that default local/spec review should not include
  bug/correctness agents.
- User wants default local/spec review to use `gpt-5.4` spec and intent
  judgment reviewers.
- User wants bug/correctness reviewers to run whenever code-producing work is
  reviewed, at appropriate times chosen by the workflow.
- User agreed review should run at proof-worthy boundaries rather than after
  every tiny edit.
- User is unsure whether `gpt-5.4-mini` has a place in review lanes, so it
  should not be assumed as a default blocking-review model.
- User says any finding above P3 means the review loop is not clean.
- User wants separate default review profiles for local/spec review after one
  execution slice, integration review after multiple slices or phases, and
  final mission-close review before completion.
- User wants those three profiles to use `gpt-5.4` judgment lanes.
- User wants the existing `$review` skill name removed.
- User says PRD/intent and mission-close judgment can use two `gpt-5.4`
  reviewers.
- User expects planning quality may need a later deeper redesign toward more
  thorough and deliberate planning; that is related context but not yet locked
  into this review-lane mission.

## Rollout Or Migration Constraints

- Existing `$review` compatibility is intentionally not preserved; the skill
  name should be removed to avoid reviewer/orchestrator confusion.

## Remaining Low-Impact Assumptions

- Exact agent count for default local/spec `gpt-5.4` spec/intent review remains
  delegated to planning.
- Exact bug/correctness review checkpoints for code-producing work are
  delegated to planning.
- Exact runtime/Ralph enforcement design is delegated to planning.
- Exact output schema for child findings is delegated to planning.

## Feasibility Constraints

Use this section only when `lock_posture = constrained`.

- None identified yet.

## Reopen Conditions

- Reopen if `$review-loop` cannot be introduced without breaking manual review
  UX.
- Reopen if native Codex subagents cannot support findings-only child lanes
  without deadlock.
- Reopen if the six-loop cap proves incompatible with mission gate semantics.

## Provenance

### User-Stated Intent

- Dedicated reviewer agent roles may be better than letting general subagents
  invoke `$review`.
- `$review` may be intended as the parent/orchestrator review-loop skill, not
  as the child reviewer prompt.
- Different review types may need different model routing: cheap bug-finding,
  integrated correctness/intent review, and final mission-close review.
- The parent/orchestrator review-loop skill should be named `$review-loop`.
- The old `$review` skill name should be removed.
- Child reviewers should not need a skill; they can be prompted with what to
  review.
- `$review-loop` should stop when clean or after six consecutive review loops,
  then route to replan if findings persist.

### Repo-Grounded Facts

- `.codex/skills/review/SKILL.md` currently instructs fresh read-only review
  context and also writeback through `record-review-outcome`.
- `.codex/skills/internal-orchestration/SKILL.md` says child agents do bounded
  work only while the parent owns mission truth.
- `docs/codex1-prd.md` says blocking review needs a fresh read-only reviewer
  thread or reviewer role bound to a review bundle.

### Codex Clarifying Synthesis

- The likely product split is parent-owned review orchestration plus
  purpose-built child reviewer roles that return findings only.
- Prompt-only rules may not be enough if child lanes can still be blocked by
  parent Ralph gates or mutate mission artifacts.
- The user delegates the exact enforcement architecture to planning, but the
  required behavior is clear: child reviewers should not be in a Ralph
  keep-going loop and should not own writeback authority.
- Codex should think through any extra review lane types during planning, but
  the baseline model-routing intent is already human-owned: default
  local/spec, PRD/intent, integration intent, and mission-close judgment use
  `gpt-5.4`; code bug/correctness review uses `gpt-5.3-codex` where
  appropriate.
- The three major review scopes should not collapse into one generic review
  profile; they need separate defaults.
