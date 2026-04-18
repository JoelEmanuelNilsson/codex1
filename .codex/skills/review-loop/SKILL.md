---
name: review-loop
description: Parent/orchestrator review-loop workflow for Codex1. Use when a mission reaches a blocking review gate, a proof-worthy review boundary, or mission-close review and Codex must orchestrate findings-only reviewer agents until clean or until the six-loop cap requires replan.
---

# Review Loop

Use this public skill as the parent-owned review orchestration surface.

`$review-loop` is not a child reviewer prompt, and it is not a parent-local
review mode. Child reviewer agents are findings-only lanes: they receive a
bounded review brief and return either `NONE` or findings with evidence refs.
They do not invoke skills, clear gates, write mission artifacts, or decide
mission completion.

Substantive review judgment belongs only to reviewer agents. The parent may
explore context, prepare snapshots and briefs, choose profiles, spawn reviewer
lanes, aggregate outputs, detect contamination, route repair/replan, and record
durable writeback. The parent must not substitute its own code, spec, intent,
integration, or mission-close judgment for reviewer-agent judgment, even for a
small, obvious, high-context, or reviewer-system change.

## Entry Rule

Review must judge a fresh bundle or proof-worthy boundary, not the writer's full
transcript.

Deterministic backend:

- begin a parent review loop with
  `CODEX1_PARENT_LOOP_BEGIN=1 codex1 internal begin-loop-lease` using
  `mode = "review_loop"` before relying on Ralph continuation
- use `codex1 internal compile-review-bundle` to create the bounded review
  contract when a bundle is owed
- use `codex1 internal validate-review-bundle` before trusting a review bundle
- use `codex1 internal record-review-outcome` for parent-owned ledger writeback,
  gate updates, and review closeout; accepted outcomes must cite real
  reviewer-output inbox artifacts such as
  `reviewer-output:<bundle-id>:<output-id>` or an explicit contaminated/replan
  reason
- child reviewers may write only the bounded reviewer-output inbox with
  `codex1 internal record-reviewer-output`; reviewer-output evidence proves
  reviewer judgment but never authorizes a child lane to call
  `record-review-outcome`

Use `internal-orchestration` to spawn bounded reviewer agents. The parent thread
keeps mission truth, final synthesis, gate writeback, repair/replan routing, and
completion judgment.

## Ralph Lease

`$review-loop` is a parent/orchestrator loop. Acquire or refresh a parent lease
before autonomous review continuation:

```json
{
  "mission_id": "<mission-id>",
  "mode": "review_loop",
  "owner": "parent-review-loop",
  "reason": "User invoked $review-loop or a blocking review gate is active."
}
```

Keep the returned parent loop authority token in the parent thread only. Use it
as `CODEX1_PARENT_LOOP_AUTHORITY_TOKEN` when calling parent-only mutation
commands during the review loop, including review-bundle compilation,
review-truth capture, review-evidence capture, and review outcome writeback.
Do not put that token in reviewer prompts, notes, evidence snapshots, or child
environment variables.
Do not pass `CODEX1_PARENT_LOOP_BEGIN=1` to reviewer lanes; that environment
capability is only for parent loop entry.

Child reviewer agents never acquire Ralph leases and must be allowed to stop
normally. The parent handles missing, partial, or invalid reviewer outputs.

## Review Profiles

Use these profiles when selecting reviewer lanes. The parent may add a narrower
lane only when the review boundary justifies it, but it must not weaken the
locked model-routing or cleanliness rules.

| Profile | When to use | Default lanes | Model | Blocking purpose |
| --- | --- | --- | --- | --- |
| `local_spec_intent` | after one spec or phase reaches a proof-worthy boundary | spec/intent judgment | `gpt-5.4` | judge whether the slice satisfies the spec and user intent |
| `integration_intent` | after multiple related specs, phases, or surfaces are combined | integration/intent judgment | `gpt-5.4` | judge cross-slice coherence, duplicated contracts, and end-goal fit |
| `mission_close` | before terminal mission completion | two PRD/intent/mission-close judgment reviewers | `gpt-5.4` | judge the integrated mission against the lock, blueprint, proof, and closeout bar |
| `code_bug_correctness` | after code-producing execution slices at proof-worthy checkpoints | bug/correctness reviewers | `gpt-5.3-codex` | find code defects, regressions, unsafe edge cases, and implementation mistakes |

Do not use `gpt-5.4-mini` as a default blocking-review model unless a later
plan explicitly gives it a narrow support-only role that cannot weaken review
quality.

## Cleanliness And Severity

Reviewer findings must use this severity scale:

- `P0`: critical breakage, data loss, security exposure, or mission-fatal
  contract violation
- `P1`: high-confidence bug or contract failure that blocks the reviewed target
  from being considered correct
- `P2`: important correctness, integration, evidence, or operability issue that
  should block clean review
- `P3`: non-blocking hardening, polish, or follow-up that should be recorded but
  does not block cleanliness by default

The latest loop is clean only when there are no `P0`, `P1`, or `P2` findings.
Any `P0`, `P1`, or `P2` finding means the loop is not clean.

## Child Output Schema

Every child reviewer must return exactly one of these shapes:

```text
NONE
```

or:

```json
{
  "findings": [
    {
      "severity": "P1",
      "title": "Short finding title",
      "evidence_refs": ["path/or/artifact:line"],
      "rationale": "Why this blocks the reviewed target.",
      "suggested_next_action": "Repair, replan, or rerun a specific review profile."
    }
  ]
}
```

Child reviewers may include `P3` findings, but the parent must keep P3 separate
from blocking P0/P1/P2 aggregation.

## Loop State

The parent must track consecutive non-clean loops for the same reviewed target
or boundary. A loop is non-clean when the parent aggregates at least one P0, P1,
or P2 finding from the latest reviewer outputs.

- loop count starts at `1` for the first review wave after a proof-worthy
  boundary
- targeted repair does not reset the count unless the reviewed target or
  governing contract changes materially
- a clean latest loop resets the count for that boundary
- six consecutive non-clean loops route to `internal-replan`

## Workflow

1. Resolve the active mission and review boundary from durable artifacts.
2. Validate package and review-bundle freshness before launching reviewer lanes.
3. Capture a parent-owned review truth snapshot with
   `codex1 internal capture-review-truth-snapshot` before launching child
   reviewers. Keep the returned writeback authority token in the parent thread
   only; do not write it into prompts, child artifacts, notes, or reviewer
   evidence. A review bundle has one parent writeback authority capture; if a
   wave needs a new authority token, compile a fresh review bundle instead of
   recapturing the same bundle.
4. Capture a frozen child-review brief with
   `codex1 internal capture-review-evidence-snapshot` and hand reviewer lanes
   only that child evidence snapshot before live repo paths. Do not give child
   reviewers the full parent-held `review_truth_snapshot`; it is a writeback
   guard for the parent, not review evidence for children.
5. Select the review profile from the boundary:
   - code-producing execution slice
   - local/spec completion
   - integration after related slices or phases
   - mission-close review
   - targeted repair re-review
6. Spawn findings-only reviewer agents with bounded read-only briefs. Use
   read-only/explorer-style reviewer lanes with `fork_turns` set to `none` for
   blocking reviews; do not use mutation-capable worker/default lanes as
   reviewers.
7. Have each reviewer lane persist exactly its bounded result with
   `codex1 internal record-reviewer-output`. `wait_agent` is only mailbox-edge
   waiting; it is not proof of completion, and chat text alone is not durable
   reviewer evidence.
8. Submit the captured `review_truth_snapshot` when recording the parent-owned
   review outcome. If mission truth changed during child review execution,
   treat the wave as contaminated and do not clear the gate.
   The writeback caller identity must be parent/orchestrator-owned; if a
   reviewer-lane-like identity attempts writeback, treat that as contamination
   and replan rather than accepting the result.
   A repo-visible review truth snapshot without the parent-held token is not
   enough to clear the gate, and arbitrary `reviewer-output:*` strings are not
   enough; writeback must cite existing reviewer-output inbox artifacts that
   were recorded after the parent truth snapshot was captured.
9. Aggregate findings by severity, evidence, and duplicate root cause. The
   aggregate must cite reviewer-agent output refs; parent-only judgment is not
   review evidence.
10. Before recording a clean outcome, confirm every required reviewer profile
   for the bundle has durable reviewer-output evidence. For code-producing
   slices with correctness review, a clean outcome requires at least one
   code/bug/correctness reviewer-output and at least one spec/intent/proof
   reviewer-output. Missing required lane output is a contaminated or blocked
   review wave, not `NONE`.
11. If the latest loop has no `P0`, `P1`, or `P2` findings and required lane
   coverage is complete, record the clean parent-owned review outcome with the
   snapshot guard.
12. If `P0`, `P1`, or `P2` findings remain and the loop count is below six,
   route to targeted repair and rerun only the relevant review profile.
13. If six consecutive review loops still find `P0`, `P1`, or `P2` issues,
   route to `internal-replan` instead of continuing indefinitely.
14. Before mission completion, require a mission-close review bundle that checks
   the integrated outcome against the lock, blueprint invariants, cross-spec
   claims, and mission-level proof rows.

## Child Reviewer Contract

Every child reviewer prompt must say:

- review only the assigned bundle, scope, or evidence
- return `NONE` or findings with severity, evidence refs, and concise rationale
- do not invoke `$review-loop` or any other skill
- do not call `record-review-outcome`
- write only the bounded `record-reviewer-output` artifact for your result
- do not clear gates, write mission artifacts, or decide completion
- do not convert your own `NONE` or findings into parent-owned review writeback
- do not request, read, echo, or persist the parent writeback authority token
- do not treat parent mission blockers as a reason to withhold the review result

## Review Rules

- Review is mandatory before mission completion.
- Passing per-spec review is necessary but not sufficient for mission
  completion.
- A mission may not close while any required review gate is open, stale, or
  failed.
- Review must not clear work against stale governing context after replan or
  resume.
- Review must not clear work when the parent-owned review truth snapshot detects
  child-lane mutation of gates, closeouts, state, review ledgers, specs,
  receipts, bundles, or mission-close artifacts.
- Review must not clear work when required reviewer lanes did not persist
  bounded reviewer-output artifacts, even if another lane returned `NONE`.
- `P0`, `P1`, or `P2` findings mean not clean.
- `P3` findings are non-blocking by default.

## Must Not

- let the last writer self-clear blocking review
- let the parent/orchestrator self-review or self-clear any review boundary
- let child reviewers mutate mission truth
- let child reviewers invoke `$review-loop`
- let child reviewers self-clear review gates by citing their own
  `reviewer-output:<lane>` evidence
- pass the parent writeback authority token to reviewer lanes or child-readable
  files
- accept missing proof rows as "close enough"
- collapse broad contradictions into local nits
- continue review/fix/review beyond six consecutive non-clean loops
- mark the mission complete without a clean mission-close review when one is
  required
- approve on the strength of summaries when the bundle is missing the raw
  evidence it claims to judge

## Return Shape

Leave a durable parent-owned review disposition with explicit finding classes,
evidence refs, loop count, and the next required branch: continue, repair,
replan, needs user, hard blocked, or complete.
