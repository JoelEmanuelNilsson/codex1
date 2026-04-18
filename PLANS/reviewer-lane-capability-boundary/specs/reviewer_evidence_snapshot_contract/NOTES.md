# Spec Notes

- Mission id: `reviewer-lane-capability-boundary`
- Spec id: `reviewer_evidence_snapshot_contract`

Use this file for bounded local notes, spike observations, or drafting scratch
that supports the spec but does not override it.

## Active Notes

- Added a frozen `ReviewEvidenceSnapshot` contract and commands:
  `capture-review-evidence-snapshot` and
  `validate-review-evidence-snapshot`.
- Evidence snapshots are written under
  `.ralph/missions/<mission-id>/review-evidence-snapshots/<bundle-id>.json`
  and include bundle bindings, source package id, governing fingerprints, proof
  rows, receipts, changed-file context, reviewer instructions, evidence refs,
  and the review truth snapshot.
- Validation rejects snapshots that omit proof rows, receipts, changed-file
  context, evidence refs, mismatch source-bundle proof/evidence arrays, or
  findings-only/no-mutation reviewer instructions.
- Review truth snapshots skip `review-evidence-snapshots/` so parent-created
  frozen briefs do not self-contaminate the mutation guard before children run.
- Mission-close evidence snapshots now carry and validate mission-level proof
  rows, cross-spec claim refs, visible artifact refs, deferred/descoped refs,
  and open finding summaries instead of requiring spec-local proof rows.
- `$review-loop`, `internal-orchestration`, runtime backend docs, and
  Multi-Agent docs now prefer frozen evidence snapshots before live mutable
  repo paths for child reviewer briefs.

## Caution

If a note changes the actual contract, move that change into `SPEC.md` or the
appropriate higher-layer artifact instead of letting this file become hidden
truth.
