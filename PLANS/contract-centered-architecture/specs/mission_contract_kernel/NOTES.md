# Spec Notes

- Mission id: `contract-centered-architecture`
- Spec id: `mission_contract_kernel`

Use this file for bounded local notes, spike observations, or drafting scratch
that supports the spec but does not override it.

## Active Notes

- Introduced `crates/codex1-core/src/mission_contract.rs` as the native kernel for mission snapshot derivation and closeout legality.
- Recast `RalphState` into a compatibility alias over `MissionContractSnapshot` so cached machine state remains stable while authority moves into the kernel.
- Routed closeout validation, interrupted-cycle recovery, contradictory-cycle state, orphan active-cycle recovery, and stop-decision semantics through the new kernel/projection boundary.
- Fixed a false `program_blueprint_fingerprint_drift` resume path by comparing closeout blueprint fingerprints against the full blueprint contract fingerprint, not the markdown-only fingerprint.

## Caution

If a note changes the actual contract, move that change into `SPEC.md` or the
appropriate higher-layer artifact instead of letting this file become hidden
truth.
