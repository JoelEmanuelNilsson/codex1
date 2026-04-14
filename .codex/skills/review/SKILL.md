---
name: review
description: Mandatory review workflow for Codex1. Use when the user invokes $review, when execution reaches a blocking review gate, or when a mission-close review bundle is ready.
---

# Review

Use this public skill to perform blocking spec review or final mission-close
review from a bounded review bundle.

## Entry Rule

Review must judge a fresh bundle, not the writer's full transcript.

Deterministic backend:

- use `codex1 internal compile-review-bundle` to create the bounded review
  contract
- use `codex1 internal validate-review-bundle` before trusting the bundle
- use `codex1 internal record-review-result` for ledger writeback, gate updates,
  and review closeout

Use a fresh read-only reviewer context. If subagents help, route that through
`internal-orchestration` so the reviewer remains independent from the last
write-capable context.

## Workflow

1. Bind the reviewer to the exact governing lock, blueprint, and spec revisions
   or fingerprints that the work claims to satisfy.
2. Confirm the bundle is fresh. If governing context changed materially,
   supersede the old bundle and require a new one.
3. Judge with the mandatory review lenses:
   - spec conformance
   - correctness and regression
   - interface compatibility
   - safety, security, and policy
   - evidence adequacy
   - operability, rollback, and observability
4. Classify findings honestly using the mission vocabulary:
   - `B-Arch`
   - `B-Spec`
   - `B-Proof`
   - `NB-Hardening`
   - `NB-Note`
5. Update visible review history:
   - `PLANS/<mission-id>/REVIEW-LEDGER.md`
   - `PLANS/<mission-id>/specs/<workstream-id>/REVIEW.md` when the review is
     spec-local
6. If blocking findings remain, route to repair or replan. If a broader
   contradiction is discovered, invoke `internal-replan` instead of softening
   the review.
7. Before mission completion, require a mission-close review bundle that checks
   the integrated outcome against the lock, blueprint invariants, cross-spec
   claims, and mission-level proof rows.

## Review Rules

- Review is mandatory.
- Passing per-spec review is necessary but not sufficient for mission
  completion.
- A mission may not close while any required review gate is open, stale, or
  failed.
- Review must not clear work against stale governing context after replan or
  resume.

## Must Not

- let the last writer self-clear blocking review
- accept missing proof rows as "close enough"
- collapse broad contradictions into local nits
- mark the mission complete without a clean mission-close review when one is
  required

## Return Shape

Leave a durable review disposition with explicit finding classes, evidence refs,
and the next required branch: continue, repair, replan, needs user, hard
blocked, or complete.
