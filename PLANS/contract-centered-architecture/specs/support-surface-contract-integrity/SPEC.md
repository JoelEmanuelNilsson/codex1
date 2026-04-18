---
artifact: workstream-spec
mission_id: contract-centered-architecture
spec_id: support-surface-contract-integrity
version: 1
spec_revision: 2
artifact_status: superseded
packetization_status: runnable
execution_status: not_started
owner_mode: solo
blueprint_revision: 2
blueprint_fingerprint: sha256:fdf15136075425675683540a46b00f5000d4da88d92b7620060caf564427f061
spec_fingerprint: null
replan_boundary:
  local_repair_allowed: false
  trigger_matrix:
  - trigger_code: write_scope_expansion
    reopen_layer: blueprint
  - trigger_code: interface_contract_change
    reopen_layer: blueprint
  - trigger_code: dependency_truth_change
    reopen_layer: blueprint
  - trigger_code: proof_obligation_change
    reopen_layer: blueprint
  - trigger_code: review_contract_change
    reopen_layer: blueprint
  - trigger_code: protected_surface_change
    reopen_layer: mission_lock
  - trigger_code: migration_rollout_change
    reopen_layer: blueprint
  - trigger_code: outcome_lock_change
    reopen_layer: mission_lock
---

# Workstream Spec

## Purpose

Collapse duplicated support-surface contract handling into one canonical, reversible, validated path so setup, restore, uninstall, and qualify all agree on what is managed, what is reversible, and what counts as drift.

## In Scope

- Centralize backup-manifest and path-containment validation for the support-surface commands.
- Make setup emit manifest truth that restore and uninstall can consume without lossy revalidation drift.
- Make qualify report the same support-surface contract state that setup and uninstall enforce.

## Out Of Scope

- Ralph closeout, resume, and selection-state semantics.
- Planning-package, review-bundle, or execution-graph changes.
- Any hidden wrapper-runtime orchestration or second machine-side truth source.

## Dependencies

- Outcome Lock revision 1.
- The current mission path layout in `paths.rs`.
- The existing artifact/frontmatter and fingerprint contract in `artifacts.rs`.
- The current support-surface inspection and trust checks in the CLI command layer.

## Touched Surfaces

- `crates/codex1/src/commands/setup.rs`
- `crates/codex1/src/commands/restore.rs`
- `crates/codex1/src/commands/uninstall.rs`
- `crates/codex1/src/commands/qualify.rs`
- `crates/codex1-core/src/artifacts.rs`
- `crates/codex1-core/src/paths.rs`

## Read Scope

- `crates/codex1/src/commands/setup.rs`
- `crates/codex1/src/commands/restore.rs`
- `crates/codex1/src/commands/uninstall.rs`
- `crates/codex1/src/commands/qualify.rs`
- `crates/codex1-core/src/artifacts.rs`
- `crates/codex1-core/src/paths.rs`

## Write Scope

- `crates/codex1/src/commands/setup.rs`
- `crates/codex1/src/commands/restore.rs`
- `crates/codex1/src/commands/uninstall.rs`
- `crates/codex1/src/commands/qualify.rs`
- `crates/codex1-core/src/artifacts.rs`
- `crates/codex1-core/src/paths.rs`

## Interfaces And Contracts Touched

- backup manifest schema and restore actions
- support-surface install/uninstall invariants
- qualify support-surface reporting and trust gating
- reversible setup manifests and exact rollback semantics

## Implementation Shape

Introduce one shared support-surface contract layer and route the CLI commands through it instead of keeping per-command copies of the same manifest and path rules. Keep the mutation model reversible, classify drift explicitly, and preserve the current observational Stop-handler policy.

## Proof-Of-Completion Expectations

- Setup, restore, and uninstall round-trip the same manifest truth without path drift or unsupported restore behavior.
- Path escape, symlink replacement, and unsupported restore actions remain rejected.
- Qualify reports the same support-surface state that setup and uninstall enforce.
- `cargo build -p codex1`
- `cargo test -p codex1`
- `cargo fmt --all --check`

## Non-Breakage Expectations

- Existing AGENTS scaffold normalization still preserves observational Stop hooks.
- Existing trusted-repo checks and user-level hook authority checks still block unsafe setup.
- Existing backup restore and rollback semantics remain reversible and exact.
- No new path traversal or symlink overwrite path appears.

## Review Lenses

- contract integrity
- path safety
- reversibility
- false-completion resistance
- evidence quality

## Replan Boundary

| Trigger code | Reopen layer |
| --- | --- |
| write_scope_expansion | blueprint |
| interface_contract_change | blueprint |
| dependency_truth_change | blueprint |
| proof_obligation_change | blueprint |
| review_contract_change | blueprint |
| protected_surface_change | mission_lock |
| migration_rollout_change | blueprint |
| outcome_lock_change | mission_lock |

## Truth Basis Refs

- `PLANS/contract-centered-architecture/OUTCOME-LOCK.md:14-23`
- `crates/codex1/src/commands/setup.rs:131-260`
- `crates/codex1/src/commands/restore.rs:70-334`
- `crates/codex1/src/commands/uninstall.rs:108-396`
- `crates/codex1/src/commands/qualify.rs:42-130`

## Freshness Notes

- This slice is current for the locked mission on `2026-04-15`.
- The selected route is intentionally narrow so the next execution-package gate can prove one recurring seam cleanly before later layers are sequenced.

## Support Files

- `REVIEW.md` will record the spec review context.
- `NOTES.md` will hold non-authoritative local notes if the slice needs them.
- `RECEIPTS/` will store proof receipts.
