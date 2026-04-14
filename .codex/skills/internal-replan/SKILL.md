---
name: internal-replan
description: Internal Codex1 replan workflow. Use when execution, review, validation, or resume reconciliation proves that local repair is no longer enough and a lock, blueprint, or execution-package layer must reopen.
---

# Internal Replan

This is an internal skill. Do not present it as a normal user-facing step.

Use it when current evidence proves that the governing contract must reopen.

## Inputs

Start from the contradiction itself:

- the violated assumption or contract
- the exact governing revision or fingerprint that was violated
- the evidence refs that proved the mismatch
- the current selected target and phase

## Reopen Decision Tree

Choose the smallest honest reopen layer:

1. Reopen the Outcome Lock when desired outcome, protected surfaces, success
   measures, unacceptable tradeoffs, or autonomy boundary changed or became
   impossible.
2. Reopen the blueprint when architecture, migration posture, proof matrix,
   review contract, or critical path changed.
3. Reopen execution-package truth when the route still stands but dependency,
   wave, or packaged scope truth changed materially.
4. Keep it local only when the governing contract still stands and the problem
   stays inside the declared local repair allowance.

## Workflow

1. Bind the contradiction to the exact governing revision it violated.
2. Decide the reopen layer using the tree above.
3. Preserve valid work aggressively. Replanning must salvage correct specs,
   receipts, and evidence whenever they still match the reopened contract.
4. Update the reopened visible artifacts honestly:
   - `OUTCOME-LOCK.md` when mission truth changed
   - `PROGRAM-BLUEPRINT.md` when route truth changed
   - affected `SPEC.md` files when local execution contracts changed
   - `REPLAN-LOG.md` for every non-local replan
5. Mark superseded truth as superseded rather than mutating history silently.
6. If implementation support exists, ensure stale execution packages, writer
   packets, and review bundles are invalidated.

Deterministic backend:

- write the contradiction first with `codex1 internal record-contradiction`
- then reopen the smallest honest layer and finish with an explicit closeout or
  helper command that writes one

## Replan Rules

- blanket discard is disallowed unless explicitly justified
- no valid evidence may vanish silently
- preserved work, invalidated work, and why each changed must be written down
- execution must not continue past a blueprint or mission-lock contradiction
  until the required reopen is complete

## Return Shape

Leave an explicit reopen result plus artifact updates that make the preserved
and invalidated truth visible to the next cycle.
