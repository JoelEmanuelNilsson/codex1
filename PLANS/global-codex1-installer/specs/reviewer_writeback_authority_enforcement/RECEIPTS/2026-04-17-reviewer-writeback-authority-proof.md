# Reviewer Writeback Authority Proof

- Mission: `global-codex1-installer`
- Spec: `reviewer_writeback_authority_enforcement`
- Source package: `fedd60de-6aa7-412e-ae5b-09a50d2f3d0a`

## Proof Rows

- `cargo test -p codex1 --test runtime_internal`
  - Result: 53 passed, 0 failed.
  - Proves review truth snapshot token enforcement, reviewer-output inbox validation, required review-lane coverage, stale bundle handling, mission-close lane coverage, stale mission-close repair routing, and the writeback authority lifecycle regressions.
- `cargo test -p codex1-core --lib`
  - Result: 90 passed, 0 failed.
  - Proves mission-close spec inclusion, mission-level repackaging stale-gate behavior, execution-package drift tolerance for completed specs, the restored stale historical execution-package regression, and runtime unit coverage remain green.
- `cargo test -p codex1 --test runtime_internal review_writeback_requires_parent_loop_authority_even_without_existing_lease -- --nocapture`
  - Result: passed.
  - Proves review truth capture is rejected without active parent loop authority even when no lease exists.
- `cargo test -p codex1 --test runtime_internal begin_loop_lease_requires_parent_begin_authority_without_existing_lease -- --nocapture`
  - Result: passed.
  - Proves a no-lease caller cannot mint parent loop authority unless the parent begin authority is explicitly present.
- `cargo test -p codex1 --test runtime_internal single_generic_reviewer_output_cannot_satisfy_all_required_lanes -- --nocapture`
  - Result: passed.
  - Proves one generic legacy reviewer output cannot satisfy both code/correctness and spec/intent required lane coverage.
- `cargo test -p codex1 --test runtime_internal multiple_generic_reviewer_outputs_cannot_satisfy_distinct_required_lanes -- --nocapture`
  - Result: passed.
  - Proves multiple generic `review_*` outputs cannot satisfy distinct required lane/profile evidence.
- `cargo test -p codex1 --test runtime_internal parent_loop_authority_blocks_child_mission_truth_mutations -- --nocapture`
  - Result: passed.
  - Proves an active parent loop lease with parent authority blocks child-style calls to pause/clear/rebegin the lease and parent-only mutation commands, including review truth capture, review bundle compilation, and review outcome writeback without `CODEX1_PARENT_LOOP_AUTHORITY_TOKEN`.
- `cargo test -p codex1 --test runtime_internal review_writeback -- --nocapture`
  - Result: 6 passed, 0 failed.
  - Proves parent writeback requires the transient token, rejects token mismatch, rejects remint for the same bundle, rejects truth snapshots captured after reviewer output, and preserves normal parent-owned clean writeback.
- `cargo test -p codex1 --test qualification_cli delegated_review_authority_gate_proves_docs_and_runtime_rejections -- --nocapture`
  - Result: passed.
  - Proves the public delegated-review qualification gate covers missing reviewer evidence, missing truth snapshots, remint rejection, post-output capture rejection, and doc-surface requirements.
- `cargo test -p codex1 --test qualification_cli`
  - Result: 38 passed, 0 failed.
  - Proves the broader setup/init, doctor, restore/uninstall, qualification, and delegated-review smoke surfaces remain green.
- `cargo test -p codex1`
  - Result: 71 unit tests passed, 38 `qualification_cli` integration tests passed, 53 `runtime_internal` integration tests passed, 0 failed.
  - Proves the blueprint P4 full-suite package contract directly for mission-close after the final authority and restore/uninstall pruning repairs.
- `cargo fmt --all --check`
  - Result: passed.
- `cargo check -p codex1`
  - Result: passed.
- `cargo test -p codex1 --test qualification_cli restore_preserves_global_codex_home_root_after_deleting_created_files -- --nocapture`
  - Result: passed.
  - Proves global restore rollback no longer prunes the CODEX_HOME root after deleting setup-created files.
- `cargo test -p codex1 --test qualification_cli uninstall_accepts_relative_codex_home_global_setup_backup -- --nocapture`
  - Result: passed.
  - Proves global uninstall rollback no longer prunes the CODEX_HOME root after deleting setup-created files.

## Behavior Proved

- `capture-review-truth-snapshot` now returns one transient parent writeback token per review bundle and refuses to remint authority for the same bundle.
- `record-review-outcome` rejects cited reviewer-output artifacts that were recorded before the submitted parent truth snapshot capture time.
- `begin-loop-lease` now requires explicit parent begin authority before minting the first verifier-backed lease, then returns a transient parent loop authority token while persisting only a verifier; review writeback authority commands require an active verifier-backed parent loop lease and that token.
- `pause-loop-lease`, `clear-loop-lease`, and active-lease replacement now require the parent loop authority token for verifier-backed leases, so reviewer lanes cannot remove the boundary and remint authority.
- Mission-close bundle construction includes carried-forward active completed specs instead of only specs rewritten at the latest blueprint revision.
- Mission-close clean review with correctness scope requires durable code/correctness and spec/intent/proof reviewer-output lanes.
- Stale mission-close review gates from abandoned close attempts no longer mask the active repair review gate once the mission has routed back into execution/review repair.
- `mission_close_review_ignores_stale_historical_execution_packages` now runs its integrated setup and mission-close assertions instead of returning early.
- Required review-lane coverage uses distinct, lane-specific reviewer-output artifacts per required lane; generic `review_*` outputs can no longer satisfy required lanes.
- A repo-visible persisted truth snapshot still lacks the plaintext token and cannot clear a review gate.
- Parent-owned writeback still works when the parent captures truth once, launches reviewer lanes, cites durable reviewer-output refs, and submits the original parent-held snapshot.
- Reviewer lanes remain able to persist bounded `record-reviewer-output` artifacts without gaining gate, ledger, closeout, or mission-completion authority.
- Restore/uninstall skill-directory pruning is now scope-aware: project-scope backup entries stop at the repo root, and user-scope global setup entries stop at CODEX_HOME instead of walking toward an unrelated repo root or parent directory.
