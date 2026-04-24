# 07 Review, Repair, And Replan Contract

This file is canonical for review-loop behavior.

The purpose of review is to catch real blockers without letting review loops
expand scope forever.

The core rule:

```text
A review finding is an observation, not work.
Only accepted_blocking findings become work.
```

## Post-Lock Autonomy

After mission lock, Codex1 should stay autonomous.

The user has already clarified the destination. `OUTCOME.md` is ratified and
`PLAN.yaml` is locked. From that point forward, ordinary product ambiguity,
implementation trouble, review dirtiness, and destructive repo edits should not
turn into user homework.

Post-lock Codex1 should:

- Make the best conservative decision that preserves the locked outcome.
- Record assumptions and tradeoffs.
- Repair accepted blockers within budget.
- Replan autonomously when repair is not working.
- Continue until the locked mission is achieved or intentionally paused.

Post-lock Codex1 should not ask the user to resolve:

- Product details missing from `OUTCOME.md`.
- Business rules not explicitly specified.
- Incompatible UX details that can be resolved by preserving the locked outcome.
- Destructive repo changes that are covered by Git, proof, and validation.
- Ordinary test/tool/dependency problems.
- Repeated dirty reviews.

`needs_user` is a pre-lock clarify concept or an explicit `$interrupt`
conversation, not a normal post-lock execution state.

## Review Boundaries

A review boundary is the exact thing being reviewed.

Examples:

- One normal-mode step.
- One graph task.
- One integration slice.
- One mission-close boundary.
- One repair diff for previously accepted blockers. Repair-diff reviews inherit
  the original boundary's repair counter; they do not create a fresh budget.

Every review must name its boundary:

```yaml
boundary_id: RB-T4-1
target:
  tasks: [T4]
  files:
    - crates/codex1/src/status/**
  proof:
    - cargo test -p codex1 status_contract
boundary_revision: 42
plan_digest: sha256:...
```

Reviewers must only review the boundary they were given. They can mention
out-of-scope concerns, but those concerns do not become blockers unless the main
thread accepts them against the locked outcome.

## Raw Review Output

Raw reviewer output should follow official Codex review shape:

```json
{
  "findings": [
    {
      "title": "[P1] Incorrect ready-wave derivation when dependency is superseded",
      "body": "The implementation treats superseded dependencies as incomplete, so downstream tasks never become ready.",
      "confidence_score": 0.86,
      "priority": 1,
      "code_location": {
        "absolute_file_path": "/abs/path/src/status.rs",
        "line_range": { "start": 120, "end": 135 }
      }
    }
  ],
  "overall_correctness": "patch is incorrect",
  "overall_explanation": "The ready-wave projection can deadlock a valid graph.",
  "overall_confidence_score": 0.82
}
```

The raw review is immutable audit. It does not mutate mission truth by itself.

## Finding Lifecycle

Every raw finding is normalized into a Codex1 finding record.

Recommended statuses:

```text
observed
accepted_blocking
accepted_deferred
rejected
duplicate
stale
fixed_claimed
verified_fixed
still_open
```

Keep the lifecycle small. Do not create a large issue-tracker inside Codex1.

Minimal finding record:

```yaml
schema_version: codex1.finding.v1
id: F-001
source_review_id: R-004
raw_finding_index: 0
boundary_id: RB-T4-1
applies_to_revision: 42
fingerprint: sha256:...
priority: 1
confidence_score: 0.86
status: accepted_blocking
adjudication:
  decision: accepted_blocking
  reason: Violates locked acceptance criterion AC-3.
  decided_at_revision: 43
repair:
  repair_round: 0
  max_repair_rounds: 2
```

## Triage Rules

Raw findings must be triaged before they can affect status.

A finding becomes `accepted_blocking` only when all are true:

- It is inside the current review boundary, or it is a direct regression caused
  by the reviewed change.
- It maps to locked outcome, locked plan, acceptance criteria, proof
  requirement, or a real P0/P1 correctness failure.
- It has concrete evidence.
- The main thread agrees it would make the current boundary unsafe to mark clean.

A finding becomes `accepted_deferred` when:

- It is real but not required to satisfy the locked outcome.
- It is cleanup, polish, future hardening, observability, or broader refactor.
- It would expand the mission beyond the lock.

A finding becomes `rejected` when:

- It contradicts the locked outcome.
- It depends on an unsupported assumption.
- It asks for alternate architecture without proving a blocker.
- It is P3/nit/future work.
- It is already covered by another accepted finding.

A finding becomes `stale` when:

- Its reviewed revision has been superseded.
- The relevant task was replanned.
- The boundary no longer exists.

## Repair Required

`repair` is valid only in this exact situation:

```text
current review boundary exists
review is current, not stale
raw findings have been triaged
accepted_blocking_count > 0
repair_round < max_repair_rounds
```

Raw findings do not imply repair. Untriaged findings imply `triage_review`.
Accepted deferred findings do not imply repair. Rejected findings do not imply
repair. Stale findings do not imply repair.

Status fragment example:

This is not the full `codex1.status.v1` envelope. It shows only the review
fields relevant to this transition.

```json
{
  "verdict": "continue_required",
  "next_action": {
    "kind": "repair",
    "owner": "codex",
    "required": true,
    "autonomous": true,
    "boundary_id": "RB-T4-1",
    "accepted_blocking_count": 2,
    "repair_round": 1,
    "max_repair_rounds": 2
  }
}
```

## Repair Budget

Use one simple repair counter per review boundary:

```yaml
repair_round: 0
max_repair_rounds: 2
```

Flow:

```text
review boundary
triage findings
if accepted blockers and repair_round < max:
    repair accepted blockers as one batch
    increment repair_round
    run targeted re-review
else if accepted blockers and repair_round >= max:
    replan_required
else:
    boundary clean
```

Do not count every raw review complaint. Count repair rounds for accepted
blockers in the same boundary.

Do not drip-feed one finding at a time forever. Batch accepted blockers for the
boundary, repair them together, then re-review the repair boundary.

Targeted repair re-reviews remain attached to the original boundary for budget
purposes. A repair diff may have its own packet/review ID, but it must not reset
`repair_round` or create a new two-round budget.

## New Findings After Repair

After the first repair round, new blockers have a higher bar.

They can become `accepted_blocking` only if they are:

- A regression caused by the repair.
- A missed locked acceptance criterion directly inside the same boundary.
- A real P0/P1 correctness issue.

Everything else is deferred, rejected, duplicate, or stale.

This prevents the review loop from expanding the mission every time a reviewer
looks again.

## Replan Required

Replan is the autonomous escape valve for engineering trouble.

`replan_required` is valid when:

- The current review boundary is still dirty after its repair budget.
- The same class of accepted blocker repeats across repair rounds.
- The implementation approach invalidates a locked plan assumption.
- The task boundary is too broad or incorrectly ordered.
- The dependency graph is wrong.
- The current plan cannot satisfy the locked outcome without changing task IDs,
  ownership, dependencies, or review boundaries.

Still dirty does not mean `needs_user`. After mission lock, still dirty means
Codex should replan from the locked outcome.

Status fragment example:

This is not the full `codex1.status.v1` envelope. It shows only the replan
fields relevant to this transition.

```json
{
  "verdict": "replan_required",
  "next_action": {
    "kind": "replan",
    "owner": "codex",
    "required": true,
    "autonomous": true,
    "reason": "repair_budget_exhausted",
    "boundary_id": "RB-T4-1"
  },
  "stop": {
    "allow": false,
    "reason": "block_replan_required",
    "mode": "strict",
    "message": "Codex1 says required work remains: replan after RB-T4-1 exhausted its repair budget.\nContinue that now, or use $interrupt / codex1 loop pause to stop intentionally.\nIf this is a false positive, explain briefly and stop; Ralph will not block again in this turn."
  }
}
```

## Validation

Do not create a top-level `validation_required` verdict.

Validation is part of task completion, review proof, and close checks.

Example:

```text
T4 requires proof: cargo test -p codex1 status_contract
```

If proof is missing, `codex1 task finish T4` should fail with a mechanical error:

```json
{
  "ok": false,
  "schema_version": "codex1.error.v1",
  "code": "MISSING_PROOF",
  "message": "T4 requires proof: cargo test -p codex1 status_contract."
}
```

Ralph does not need a separate validation state. Status should continue pointing
at the unfinished task or review boundary.

## Close Review

Mission-close review is a review boundary, not a public `$finish` skill. It is
required for graph, large, risky, or explicitly configured missions. Simple
normal-mode missions may close through `close check` and `close complete`
without mission-close review.

Mission-close review can pass only when:

- The locked outcome is satisfied.
- Required tasks/steps are complete.
- Required current reviews are clean.
- There are no accepted blocking findings.
- Any deferred findings are recorded as nonblocking.
- Replan is not required.

After the passed review is recorded with `codex1 close record-review`,
`codex1 close check` confirms every pre-close gate before `codex1 close
complete` may record terminal state.

`CLOSEOUT.md` is terminal evidence, but it must not create a close-check cycle.
`codex1 close complete` writes or verifies `CLOSEOUT.md`, then records terminal
state. If a closeout already exists, it must match the current state revision or
be rewritten before terminal state is recorded.

If mission-close review is dirty, apply the same rules:

- Triage raw findings.
- Repair accepted blockers within budget.
- Re-review the repair boundary.
- Replan autonomously when the repair budget is exhausted.

## Ralph Interaction

Ralph enforces process state, not quality state.

Ralph never reads raw findings and never decides whether findings are valid.

For review, repair, and close-review states, Ralph may block only on these
projected next-action kinds:

- `triage_review`
- `repair`
- `replan`
- `close_review`
- `record_close`

The full Ralph allowlist is canonical in
`06-ralph-stop-hook-contract.md`. The status projection is responsible for
deciding which action is next.

## Anti-Spiral Rules

These are product rules, not suggestions:

- Findings are observations, not work.
- Only accepted blocking findings block progress.
- Raw findings must be triaged.
- Repair reviews are targeted to accepted blockers and their repair diff.
- One repair counter per boundary.
- Default `max_repair_rounds` is 2.
- After repair budget, replan autonomously.
- After mission lock, do not ask the user to resolve ordinary product or
  implementation ambiguity.
- Do not create `blocked_external` or `validation_required` as normal verdicts.
- Do not let reviewers reopen the whole mission during repair review.
