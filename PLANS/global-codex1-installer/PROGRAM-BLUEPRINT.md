---
artifact: program-blueprint
mission_id: global-codex1-installer
version: 1
lock_revision: 1
blueprint_revision: 9
plan_level: 4
risk_floor: 4
problem_size: M
status: approved
proof_matrix:
- claim_ref: P1
  statement: Global setup does not mutate projects by default.
  required_evidence:
  - CLI fixture test
  review_lenses:
  - protected-surface safety
  - correctness
  governing_contract_refs:
  - OUTCOME-LOCK.md
- claim_ref: P2
  statement: Project setup remains available only via explicit codex1 init.
  required_evidence:
  - CLI fixture test
  review_lenses:
  - command UX clarity
  governing_contract_refs:
  - OUTCOME-LOCK.md
decision_obligations:
- obligation_id: DO-1
  question: Should setup and init both exist in this mission?
  why_it_matters: Determines CLI command boundary.
  affects:
  - architecture_boundary
  - execution_sequencing
  governing_contract_refs:
  - OUTCOME-LOCK.md
  review_contract_refs:
  - command UX clarity
  mission_close_claim_refs:
  - P1
  - P2
  blockingness: critical
  candidate_route_count: 2
  required_evidence:
  - user answer
  status: selected
  resolution_rationale: User selected Setup + init.
  evidence_refs:
  - MISSION-STATE.md
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
selected_target_ref: spec:reviewer_writeback_authority_enforcement
---
# Program Blueprint

## 1. Locked Mission Reference

- Mission id: `global-codex1-installer`
- Lock revision: `1`
- Lock fingerprint: `sha256:f23e8268294648091f430323e3b2b271aacde8d956deb85cd3577a917f2d2343`
- Outcome summary: Make `codex1 setup` a global machine/runtime installer with backups and doctor checks, and make `codex1 init` the explicit current-project setup command.

## 2. Truth Register Summary

| Row | Type | Statement | Evidence ref | Source type | Observation basis | Observed revision or state | Freshness | Confidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| T1 | verified_fact | Current `codex1 setup` is project-scoped and writes repo-local support surfaces. | `crates/codex1/src/commands/setup.rs` | repo | source read | working tree on 2026-04-17 | fresh | high |
| T2 | verified_fact | CLI command wiring has no public `init` command yet. | `crates/codex1/src/main.rs`; `crates/codex1/src/commands/mod.rs` | repo | source read | working tree on 2026-04-17 | fresh | high |
| T3 | verified_fact | Backup manifests already model user and project scopes and content hashes. | `crates/codex1-core/src/backup.rs` | repo | source read | working tree on 2026-04-17 | fresh | high |
| T4 | verified_fact | Oh-my-codex defaults setup scope to user/global and separates project scope. | `/Users/joel/oh-my-codex/src/cli/setup.ts` | local reference repo | source read | local clone on 2026-04-17 | fresh | high |

## 3. System Model

- Touched surfaces: public CLI dispatch, setup/init command modules, setup-related CLI tests, and later doctor/restore surfaces.
- Boundary summary: global `setup` targets user-level Codex surfaces; project `init` targets the resolved current project/repo.
- Hidden coupling summary: the current setup code combines project setup behavior and command name; the first slice separates that boundary before deeper global setup and doctor work.

## 4. Invariants And Protected Behaviors

- Global `setup` must never write repo-local `.codex`, repo-local `AGENTS.md`, or Git state by default.
- Existing user-owned `~/.codex/config.toml` and `~/.codex/hooks.json` must be backed up before content changes in later setup work.
- Existing unrelated uncommitted work in `/Users/joel/codex1` is not to be reverted.
- Project-scoped behavior must be explicit through `codex1 init`.

## 5. Proof Matrix

| Proof row | What must be proven | Evidence class | Owner | Blocking |
| --- | --- | --- | --- | --- |
| P1 | `codex1 setup` writes only user/global support surfaces and does not mutate a temp cwd project. | CLI integration test | implementation | yes |
| P2 | `codex1 init` preserves former project-scoped setup behavior behind an explicit command. | CLI integration test | implementation | yes |
| P3 | `codex1 doctor` distinguishes global setup health from project init health. | CLI integration test | implementation | yes |
| P4 | Existing test suite still passes after command split and backup changes. | `cargo test -p codex1` | implementation | yes |
| P5 | `codex1 setup` installs global managed skills, including `$close`, and `codex1 init` succeeds after global setup without adding a duplicate project Stop hook. | CLI integration test | implementation | yes |
| P6 | Clean blocking review cannot be recorded unless every required reviewer lane/profile has durable reviewer-output evidence. | runtime regression test + qualification proof | implementation | yes |
| P7 | Findings-only reviewer lanes cannot mint or use parent-owned review writeback authority. | runtime regression test + qualification proof | implementation | yes |

## 6. Decision Obligations

| Obligation id | Question | Why it matters | Blockingness | Status | Evidence refs |
| --- | --- | --- | --- | --- | --- |
| DO-1 | Should setup and init both exist in this mission? | Determines CLI command boundary and migration shape. | critical | selected: user chose setup + init | `OUTCOME-LOCK.md`; user answer |
| DO-2 | Should global setup force-normalize existing authoritative user hooks by default? | Controls protected-surface risk. | major | selected: no, require explicit force for unsafe normalization | `OUTCOME-LOCK.md` |

## 7. In-Scope Work Inventory

| Work item | Class | Why it exists | Proof / review owner | Finish in this mission? |
| --- | --- | --- | --- | --- |
| CLI command split | runnable_frontier | Expose `init` as explicit project setup and reserve `setup` for global setup. | implementation + review | yes |
| Global setup with backups | near_frontier | Make machine-level setup safe, idempotent, and reversible after command boundary is split. | implementation + review | yes |
| Doctor/restore verification updates | near_frontier | Prove global setup and project init are observable and recoverable after setup manifests are stable. | implementation + review | yes |
| Review lane completion guard | runnable_frontier | Replan after contradiction `e7198761`: clean review was accepted without required code/correctness reviewer-output evidence. | implementation + review | yes |
| Reviewer writeback authority enforcement | runnable_frontier | Replan after contradiction `0d8f3968`: mission-close reviewer lanes self-terminalized by calling parent-owned writeback. | implementation + review | yes |
| Release packaging | deferred_or_descoped | User asked for current-machine compatibility first. | none | no |

## 8. Option Set

- Option A: Rename current project-scoped setup implementation to `init`, then build a new global `setup` that reuses support-surface backup primitives for user-scope paths.
- Option B: Add a `--scope user|project` flag to existing `setup` and keep `setup` as the only public command.

Selected route: Option A.

## 9. Selected Architecture

Use a command-boundary split. First move the current repo-scoped setup logic behind `codex1 init`. Then replace `codex1 setup` with a global support-surface planner that targets `CODEX_HOME` or `~/.codex` and writes user-scope managed entries/files. Reuse manifest/transaction primitives for setup/init/restore/uninstall. Update doctor after global setup manifest shape is stable.

## 10. Rejected Alternatives And Rationale

- Keep project writes under `codex1 setup`: rejected because it repeats the failed `rotrut` behavior.
- Implement only global setup and defer `init`: rejected because the user explicitly chose `Setup + init`.

## 11. Migration / Rollout / Rollback Posture

- Migration posture: preserve old project setup behavior by moving it to `codex1 init`.
- Rollout posture: local machine compatibility first; no publishing or remote release in this mission.
- Rollback posture: use support-surface manifests under `~/.codex1/backups`; restore/uninstall must use scope-aware manifest entries.

## 12. Review Bundle Design

- Mandatory review lenses: correctness, protected-surface safety, backup/restore integrity, command UX clarity, test adequacy.
- Required receipts: targeted CLI integration test output; `cargo test -p codex1` output.
- Required changed-file context: CLI command wiring, setup/init/doctor/restore/uninstall modules, backup helpers if touched, and CLI tests.
- Mission-close claims requiring integrated judgment: global setup does not mutate projects by default; project setup is explicit; backups precede mutation; doctor verifies honestly.
- Replanned lane-completion invariant: a clean review must cite durable reviewer-output artifacts for every required profile implied by the bundle/lenses. For code-producing slices with correctness review, at least one code/correctness reviewer-output and one spec/intent or proof reviewer-output are required before the parent can record a clean outcome. Missing required lane output must route to replan/blocked review, never to clean.
- Replanned writeback-authority invariant: reviewer lanes must not be able to mint or use parent-owned review writeback authority. Parent writeback must be bound to a parent-owned capability that child reviewer commands cannot create by replaying `capture-review-truth-snapshot`, spoofing reviewer ids, or citing otherwise valid reviewer-output artifacts.

## 13. Workstream Overview

| Spec id | Purpose | Packetization status | Owner mode | Depends on |
| --- | --- | --- | --- | --- |
| cli_command_split | Add explicit project init and reserve setup for global machine setup. | runnable | solo | none |
| global_setup_backup | Implement global setup with user-scope backups, global managed skills, and setup-to-init compatibility. | runnable | solo | cli_command_split |
| doctor_restore_verification | Update doctor and restore verification for global setup and project init. | runnable | solo | global_setup_backup |
| review_lane_completion_guard | Enforce required reviewer-output lane coverage before clean review writeback. | runnable | solo | doctor_restore_verification |
| reviewer_writeback_authority_enforcement | Enforce parent-only review writeback authority so reviewer lanes cannot self-clear or terminalize. | runnable | solo | review_lane_completion_guard |

## 14. Execution Graph And Safe-Wave Rules

- Graph summary: Execute serially: `cli_command_split`, then `global_setup_backup`, then `doctor_restore_verification`, then `review_lane_completion_guard`, then `reviewer_writeback_authority_enforcement`.
- Safe-wave rule 1: Do not edit `restore`/`uninstall` until the manifest scope shape for global setup is known.
- Safe-wave rule 2: Do not run setup against the real user home during tests; use temp HOME/CODEX_HOME fixtures.
- Safe-wave rule 3: After global setup is introduced, project `init` must treat the managed user-level Codex1 Stop hook as the authoritative Ralph pipeline and must not add a second project Stop hook.

## 15. Risks And Unknowns

- Codex global guidance file discovery may differ from the assumed `~/.codex` location.
- Existing uncommitted changes in Codex1 may overlap with setup/doctor modules.
- `qualify-codex` may remain project-oriented and require careful wording.

## 16. Decision Log

| Decision id | Statement | Rationale | Evidence refs | Affected artifacts | Adopted in revision |
| --- | --- | --- | --- | --- | --- |
| D-1 | Use command split: `setup` global, `init` project. | Matches user ratification and avoids default project mutation. | `OUTCOME-LOCK.md` | blueprint, all specs | 1 |
| D-2 | Use existing project setup logic as the basis for `init`. | Lowest-risk way to preserve capability while changing default behavior. | `crates/codex1/src/commands/setup.rs` | cli_command_split | 1 |
| D-3 | Reuse support-surface backup primitives for global setup. | Existing code already models user/project scopes and content hashes. | `crates/codex1-core/src/backup.rs` | global_setup_backup | 1 |
| D-4 | Global setup owns global managed skill installation, including `$close`, and project `init` coexists with the global managed Stop hook. | Contradiction `5224cd13-81ff-4da8-97f2-d4bcd30dd04c` proved config/hooks-only setup was not enough for machine-wide workflow use. | `REPLAN-LOG.md`; `crates/codex1/tests/qualification_cli.rs` | global_setup_backup | 6 |
| D-5 | Review cleanliness requires machine-checked lane completion, not parent prompt discipline alone. | Contradiction `e7198761-0b56-4978-b587-2aeaa944a03e` proved a clean review could be recorded with only spec/intent reviewer output while required code/correctness review lanes never persisted output. | `REPLAN-LOG.md`; `.ralph/missions/global-codex1-installer/reviewer-outputs/666382e9-0198-4d6c-b007-2a334aafe3f2/4a5d6937-da75-4fd3-bd2d-ad8f01d15dde.json` | review_lane_completion_guard | 8 |
| D-6 | Review writeback authority must be machine-bound to the parent, not merely hidden by prompt. | Contradiction `0d8f3968-2f41-460d-9ab8-3c08660c3aac` proved mission-close reviewer lanes could mint/use writeback authority and terminalize the mission. | `REPLAN-LOG.md`; `.ralph/missions/global-codex1-installer/closeouts.ndjson:53`; `.ralph/missions/global-codex1-installer/gates.json` | reviewer_writeback_authority_enforcement | 9 |

## 17. Replan Policy

- Reopen Outcome Lock when: user changes whether setup may mutate projects by default, or publishing/cross-platform support becomes required.
- Reopen blueprint when: global Codex guidance cannot be implemented under the assumed user-level Codex location, or backup scope shape must change non-locally.
- Reopen execution package when: write scope expands outside the selected spec or tests require real-home mutation.
- Local repair allowed when: implementation details change within listed write scopes while preserving command contracts and proof matrix.
