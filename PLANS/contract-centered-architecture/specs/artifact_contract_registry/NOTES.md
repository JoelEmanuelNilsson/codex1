# Spec Notes

- Mission id: `contract-centered-architecture`
- Spec id: `artifact_contract_registry`

Use this file for bounded local notes, spike observations, or drafting scratch
that supports the spec but does not override it.

## Active Notes

- Added a core machine-readable visible-artifact text registry in `crates/codex1-core/src/artifacts.rs`.
- Routed `validate-visible-artifacts` in `crates/codex1/src/internal/mod.rs` through the registry instead of raw marker arrays.
- Added template parity tests so `README.md`, `REVIEW-LEDGER.md`, and `REPLAN-LOG.md` stay aligned with the registered contract.

## Caution

If a note changes the actual contract, move that change into `SPEC.md` or the
appropriate higher-layer artifact instead of letting this file become hidden
truth.
