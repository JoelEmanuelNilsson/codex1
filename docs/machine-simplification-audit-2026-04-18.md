# Codex1 Machine Simplification Audit

**Date:** 2026-04-18
**Scope:** contracts, Ralph loop, planning, orchestration, review — the entire "machine" surface
**Framing:** PRD `docs/codex1-prd.md` is the intent reference. This audit treats the PRD's *why* as load-bearing and the PRD's low-level prescriptions as advisory.
**Original status:** read-only deep analysis; no code was changed. Three parallel Opus 4.7 agents owned the three pillars and their findings are synthesized here. All high-stakes claims (lock reopen unimplemented, dead ExecutionPackageStatus variants, `#[cfg(test)]` bypass at runtime.rs:5619-5624, unused `ralph::GateEntry`, always-`None` `WorkstreamSpecFrontmatter.spec_fingerprint`) were verified by direct grep/read after synthesis.

**Codex1 annotation (2026-04-18):** Treat this audit as a diagnosis and backlog, not as the execution plan. Subsequent product review agreed with most findings but rejected the original roadmap ordering: the correct next work is to finish the mode/authority model as a first-class product contract before doing large structural simplification. In particular, parent-loop vs discussion vs subagent vs findings-only-reviewer authority must be promoted into PRD/skills/runtime/test truth before the authority-heavy runtime surfaces are mechanically split.

---

## 0. TL;DR — what this audit found

0. **This audit is a diagnosis, not an execution plan.** Its findings are useful, but the simplification roadmap must be reordered around the product-mode contract: parent-loop authority, human discussion/interrupt boundaries, subagent Ralph exemption, findings-only reviewer lanes, and parent-owned mission-truth writeback.
1. **The machine is substantively correct against the PRD, but ~80% of the pain is accidental structure rather than missing behaviour.** `runtime.rs` is 17,766 lines of cohesive-but-undifferentiated code. Every functional boundary that should be a module is a section comment.
2. **The anti-false-completion discipline is genuine rigor, not ceremony.** Closeout schema validation, sequence-gap detection, parent-held writeback authority token, review-wave contamination detection, required-lane coverage, and the `HaltHardBlocked non-terminal until reviewed` rule are all load-bearing and intentionally deep.
3. **Real defects exist** — most are partial PRD implementations rather than logical bugs: lock reopen is unimplemented; `ExecutionPackageStatus::Consumed`/`Superseded` exist without a canonical lifecycle decision; phase-specific `next_phase` transitions are unenforced; mission-close gate scoping needs a canonical "required gates for current mission truth" derivation instead of either source-package-only or all-history gate checks.
4. **The biggest code-smell is semantic drift across three near-identical atomic writers.** One drops `fsync_dir`, violating the PRD's durable-write contract for a small set of files.
5. **The biggest undocumented-but-load-bearing construct is the loop lease** (`.ralph/loop-lease.json`). It is the single gate that decides whether stop-hook blocks on actionable missions. The PRD does not mention it.
6. **The biggest mechanical simplification is still a 3–4 PR split of `runtime.rs` into purpose-focused modules, but not before the authority model settles.** Shallow extraction of orthogonal utilities is safe; authority-heavy review/resume/lease/gate surfaces should wait until the product-mode contract is explicit.

Priority-ranked roadmap in §7.

---

## 1. PRD intent baseline — what the machine exists to do

From PRD §1–§8 (lines 1–2400), compressed to invariants the machine must satisfy:

- **Visible mission artifacts under `PLANS/<id>/`** are the product surface: `README.md`, `MISSION-STATE.md`, `OUTCOME-LOCK.md`, `PROGRAM-BLUEPRINT.md`, `specs/<id>/SPEC.md`, `REVIEW-LEDGER.md`, `REPLAN-LOG.md`. These are read by humans and survive compaction.
- **Hidden machine state under `.ralph/missions/<id>/`** is the enforcement layer: `closeouts.ndjson`, `state.json`, `active-cycle.json`, `gates.json`, `execution-packages/`, `packets/`, `bundles/`, `reviewer-outputs/`, `review-truth-snapshots/`, `review-evidence-snapshots/`, `contradictions.ndjson`, `execution-graph.json`, `waves/`.
- **Selection + lease state under `.ralph/`**: `selection-state.json` for multi-mission arbitration, `loop-lease.json` for parent-loop authority.
- **Ralph's contract**: honest continuation, explicit closeout, phase+verdict discipline, no prose-only completion. The two failure modes to prevent are **false stop** (mission ended early while actionable work remains) and **false continue** (writer kept pushing past the point of needing review/replan/user).
- **Skills are the product surface**; the Rust crates are deterministic backend helpers. The PRD warns against `runtime-first system where machinery matters more than Codex workflow quality`. The harness should "feel like using Codex normally" with much stronger artifact discipline.

That last warning is why the 17k-line `runtime.rs` is itself a PRD-alignment concern: the machinery has grown into the centre of gravity. Every simplification below is measured against the PRD's "harness, not runtime" framing.

---

## 2. Machine surface — what exists and where

### 2.1 Contracts (visible + hidden)

| Tier | Contract | Location | Lifecycle owner |
| --- | --- | --- | --- |
| Sacred visible | `OutcomeLockFrontmatter` | `artifacts.rs:501-519`, `PLANS/<id>/OUTCOME-LOCK.md` | `initialize_mission` (runtime.rs:1297); no reopen path exists |
| Visible | `MissionStateFrontmatter` | `artifacts.rs:480-498` | `initialize_mission`; clarify owns revisions |
| Visible | `ProgramBlueprintFrontmatter` | `artifacts.rs:521-547` | `write_planning_artifacts` (runtime.rs:1568) + `next_blueprint_revision` (runtime.rs:10521) |
| Visible | `WorkstreamSpecFrontmatter` | `artifacts.rs:549-573` | `sync_planning_specs` (runtime.rs:1821) + `next_spec_revision` (runtime.rs:10507); mutated to `Packaged` at 2151–2171 |
| Hidden | `ExecutionPackage` | `runtime.rs:290-318`; `.ralph/missions/<id>/execution-packages/<pkg>.json` | `compile_execution_package` (runtime.rs:2078) + `validate_execution_package` (runtime.rs:2442) |
| Hidden | `WriterPacket` | `runtime.rs:499-514`; `packets/<packet>.json` | `derive_writer_packet` (runtime.rs:2623) |
| Hidden | `ReviewBundle` | `runtime.rs:558-597`; `bundles/<bundle>.json` | `compile_review_bundle` (runtime.rs:2753) |
| Hidden | `ReviewEvidenceSnapshot` | `runtime.rs:873-908`; `review-evidence-snapshots/<bundle>.json` | `capture_review_evidence_snapshot` (runtime.rs:3167) — child-readable |
| Hidden | `ReviewTruthSnapshot` | `runtime.rs:980-990`; `review-truth-snapshots/<bundle>.json` | `capture_review_truth_snapshot` (runtime.rs:3703) — parent-only, remint refused |
| Hidden | `ReviewerOutputArtifact` | `runtime.rs:1021-1033`; `reviewer-outputs/<bundle>/<output>.json` | `record_reviewer_output` (runtime.rs:3410) — child writeback, no side-effects |
| Hidden | `MissionGateIndex`/`MissionGateRecord` | `runtime.rs:600-628`; `gates.json` | `initial_gates` (runtime.rs:7358), `supersede_matching_gates` (runtime.rs:7425), `append_gate` |
| Hidden | `ContradictionRecord` | `runtime.rs:631-657`; `contradictions.ndjson` | `append_contradiction` (runtime.rs:4912) |
| Hidden | `ExecutionGraph` | `runtime.rs:481-489`; `execution-graph.json` | `build_execution_graph` (runtime.rs:8910) |
| Hidden | `WaveManifest` | `runtime.rs:346-356`; `waves/<wave>.json` | `build_wave_manifest` |
| Hidden | `RalphLoopLease` | `runtime.rs:170-185`; `.ralph/loop-lease.json` | `begin_ralph_loop_lease` (runtime.rs:5401); verifier-only on disk |
| Hidden | `SelectionState` | `runtime.rs:660-674`; `.ralph/selection-state.json` | `open_selection_wait` (runtime.rs:5051) |
| Continuation | `CloseoutRecord` | `ralph.rs:73-124`; `closeouts.ndjson` | `append_closeout_and_rebuild_state` (ralph.rs:473) |
| Continuation | `ActiveCycleState` | `ralph.rs:163-262`; `active-cycle.json` | removed on successful closeout; orphan branch handled |
| Continuation | `MissionContractSnapshot` / `RalphState` | `mission_contract.rs:10-55`; `state.json` | `derive_snapshot_from_closeouts`, projection only |

### 2.2 Skill surface

| Class | Skill | Owns | Parent Ralph loop? |
| --- | --- | --- | --- |
| Public | `clarify` | MISSION-STATE + OUTCOME-LOCK bootstrap | **No** (interactive intake; `$autopilot` consumes the handoff) |
| Public | `autopilot` | End-to-end branch router | Yes: `autopilot_loop` |
| Public | `plan` | BLUEPRINT + frontier SPECs + execution package | Yes: `planning_loop` |
| Public | `execute` | Execution of passed package | Yes: `execution_loop` |
| Public | `review-loop` | Parent-owned review orchestration (spec + mission-close) | Yes: `review_loop` |
| Public | `close` | Pause/clear the lease; discuss without uninstalling hooks | Lease consumer only |
| Internal | `internal-orchestration` | Bounded multi-agent help | Not lease-holding |
| Internal | `internal-replan` | Reopen lock/blueprint/execution-package on contradiction | Not lease-holding |

### 2.3 Internal CLI surface (`codex1 internal …`)

From `crates/codex1/src/internal/mod.rs` (1831 lines): 31 subcommands including `stop-hook`, `repair-state`, `validate-*`, `init-mission`, `materialize-plan`, `compile-execution-package`, `derive-writer-packet`, `compile-review-bundle`, `capture-review-truth-snapshot`, `capture-review-evidence-snapshot`, `record-reviewer-output`, `record-review-outcome`, `record-contradiction`, `append-replan-log`, `open/resolve/consume/clear-selection-wait`, `begin/pause/clear/inspect-loop-lease`, `append-closeout`, `resolve-resume`, `acknowledge-waiting-request`. These are the concrete ways skills invoke the deterministic backend.

---

## 3. How the pillars actually work

### 3.1 Contracts pillar — state machines vs PRD

**Strong alignment**:

- `OutcomeLock/Blueprint/Spec/MissionState` frontmatter parsing uses a typed `ArtifactDocument<F>` envelope (`artifacts.rs:258-300`); kind-mismatch is caught at parse time.
- `WorkstreamSpec` correctly separates three state axes (`artifact_status`, `packetization_status`, `execution_status`) with legal-combination enforcement in `validate_planning_spec_state` (`runtime.rs:7118-7150`).
- `MissionGateIndex` uses append-preserving + supersede markers, matching PRD §7's lifecycle rules.
- `CloseoutRecord` + `validate_closeout_contract` (`mission_contract.rs:94-216`) enforce the verdict × terminality × resume_mode matrix exhaustively.
- Review cluster: `ReviewTruthSnapshot` parent-held capability + `ReviewEvidenceSnapshot` child-readable + `ReviewerOutputArtifact` inbox + parent writeback authority token is a genuinely deep defence-in-depth stack.

**Partial PRD divergence**:

| PRD expectation | Code reality | Risk |
| --- | --- | --- |
| Lock reopen is first-class with revision bump (PRD §5.14, §6.26) | `LockStatus::Reopened` is never assigned anywhere. `LockStatus::Superseded` is assigned only in one narrow mapping at `runtime.rs:1334` (`ClarifyStatus::Superseded → LockStatus::Superseded` inside `initialize_mission`), which is not a lock-reopen workflow. No codepath bumps `lock_revision`. Re-invoking `initialize_mission` silently overwrites. | A silent lock mutation could pass as "lock_revision = 1" forever. **Highest-severity PRD gap in contracts pillar.** |
| `ExecutionPackageStatus {draft, ready_for_gate, passed, failed, superseded, consumed}` all meaningful (PRD §7.17) | Only `Passed` and `Failed` are ever assigned. Supersession and consumption are tracked via `gates.json`/closeouts instead. | The system has not chosen one canonical lifecycle truth: either package status is immutable validation result and dead lifecycle variants should go, or package status owns lifecycle and gates/closeouts derive from it. |
| `replan_boundary` copied consistently into Spec, Package, WriterPacket (PRD §7.25) | Spec → Package → Packet chain is enforced by `derive_package_replan_boundary` (8737–8773) and `validate_writer_packet` (2732). | ✅ OK — one of the strongest cross-artifact checks. |
| `spec_fingerprint` field on every included spec | Three different fingerprints share the name (see §3.1b). The one on `WorkstreamSpecFrontmatter` (artifacts.rs:562) is always `None`. | Dead field + invitation for drift. |
| `clarify_status` moves to `ratified` once lock ratifies (PRD §5.757-759) | No runtime enforcement; relies on caller passing correctly. | Low-severity but a tidiness gap. |
| Required fingerprint fields on all post-lock closeouts (PRD §8.2248-2269) | Schema allows `lock_revision = null`, `lock_fingerprint = null`, `blueprint_revision = null`, `summary = null` on every closeout. | Phase-sensitive enforcement would close the gap. |

**3.1b Three-meaning `spec_fingerprint`:**

1. **`WorkstreamSpecFrontmatter.spec_fingerprint: Option<Fingerprint>`** — always `None`, dead field.
2. **`IncludedSpecRef.spec_fingerprint`** (inside `ExecutionPackage.included_specs`) — full-content fingerprint of rendered SPEC.md.
3. **`ExecutionGraphNode.spec_fingerprint`** — a different "contract-only projection" computed by `execution_graph_spec_contract_fingerprint` (runtime.rs:8156).

All three are `Fingerprint` values under the same field name. A future reviewer reading a callsite cannot tell which meaning applies without tracing the type.

**3.1c Blueprint fingerprint can drift against `blueprint_revision`:**

`compute_blueprint_contract_fingerprint` (runtime.rs:8870–8889) includes the execution graph in the hash when present. `blueprint_materially_matches_existing` (runtime.rs:10538–10553) does **not** compare the graph. Consequence: a graph-only change (e.g., dependency edges re-arranged) produces a different `blueprint_fingerprint` but the same `blueprint_revision`. Downstream callers that identify revisions as `blueprint:<N>` will see two different fingerprints for the same `N`. `invalidate_post_planning_history` partly compensates by staling downstream gates — but the package file still carries `blueprint_fingerprint: <new>` / `blueprint_revision: <old>`, which is ambiguous.

### 3.2 Ralph loop pillar — continuity and false-completion defence

**The closeout → state rebuild flow** (`ralph.rs:473-554`, `append_closeout_and_rebuild_state`) is tight:

1. fs2 advisory lock on `closeouts.ndjson`.
2. Re-load + re-validate the full history; malformed **final** line is silently skipped as truncated debris (tested at `ralph.rs:847-862`).
3. Dedup-by-full-identity: same `(closeout_id, cycle_id, closeout_seq)` triggers idempotent replay (re-derive state, remove active-cycle, return).
4. Sequence invariant (`expected_seq = last.seq + 1`) and cycle/mission consistency checks.
5. Build snapshot from augmented history, append ndjson line, `file.sync_all()`, `atomic_write_json(state.json)` (temp + persist + fsync_dir), `atomic_remove_file(active-cycle.json)`, `fsync_dir(mission_dir)`, unlock.

This is a correct "one logical commit" protocol for the PRD's §8 durable-write contract.

**Stop-hook pipeline** (`run_stop_hook → resolve_stop_hook_output → resolve_resume → stop_output_from_resume_report`):

- `NoActiveMission` → allow stop
- `WaitingSelection` → emit `systemMessage = canonical_selection_request` (never block)
- `WaitingNeedsUser` → emit `systemMessage = canonical_waiting_request` (never block); ack the request if not emitted
- `ActionableNonTerminal | InterruptedCycle | ContradictoryState` → **block iff the loop lease is Active**, else advisory `systemMessage`
- `Terminal` → allow stop, with belt-and-braces `latest_closeout_is_terminal` re-check

The **`enforce_actionable = (lease.status == Active)`** gate at `runtime.rs:5374-5377` is the keystone. The PRD does not mention the lease — it's undocumented product semantics that operationalize "the main orchestrating thread is the only authority allowed to mutate canonical Ralph mission state" across independent Codex windows. Skill SKILL.md files (autopilot/plan/execute/review-loop) all correctly instruct the parent to acquire a lease before autonomous continuation; it's the safety net.

**Contradiction resume-override** (`contradiction_resume_override` at runtime.rs:5842-5943) correctly maps:

| MachineAction | ResumeStatus | Verdict | next_phase | reason_code |
| --- | --- | --- | --- | --- |
| ForceReview | ContradictoryState | ReviewRequired | review | unresolved_contradiction_force_review |
| ForceRepair | ContradictoryState | RepairRequired | execution | unresolved_contradiction_force_repair |
| YieldNeedsUser | WaitingNeedsUser | NeedsUser | discovered_in_phase | unresolved_contradiction_needs_user |
| HaltHardBlocked | ContradictoryState | ReplanRequired | replan | unresolved_contradiction_pending_hard_block_closeout |
| ForceReplan / None | ContradictoryState | ReplanRequired | replan | unresolved_contradiction_force_replan |

Crucially, `HaltHardBlocked` is **not** terminal on the fly — PRD requires a reviewed closeout before hard_blocked terminalizes. Tested at `runtime_internal.rs:3626+`.

**False-completion defences — strength matrix**:

| Defence | Location | Strength |
| --- | --- | --- |
| Closeout verdict/terminality/resume_mode triple | mission_contract.rs:144-199 | Strong (exhaustive match) |
| Sequence gap + duplicate id | ralph.rs:402-442 | Strong |
| Mission-id mixing | ralph.rs:416-427 | Strong |
| fs2 advisory lock | ralph.rs:489-551 | Medium (local FS only; fine for macOS V1) |
| Fingerprint drift detection | `current_fingerprint_findings` runtime.rs:6343-6420 | Strong |
| Interrupted-cycle override | mission_contract.rs:218-296 | Strong (clears waiting identity) |
| `contradictory_active_cycle` | ralph.rs:573-583 | Strong (forces ReplanRequired) |
| Mission-close contradiction findings | runtime.rs:5828-5839 | Strong (any unresolved contradiction blocks close) |
| Parent-auth token for mission mutation | runtime.rs:5587-5613 | Medium (no-lease-allows in production) |
| Parent-auth token for review writeback | runtime.rs:5615-5662 | Strong in production; **`#[cfg(test)]` bypass at :5619-5624 is a concern** |
| Findings-only-reviewer stop-hook bypass | internal/mod.rs:622-661 | Medium (three overlapping heuristics — path-based one is fragile) |
| Required-lane coverage for clean review | runtime.rs:4475-4512 | Strong (distinct outputs required), but lane-name substring match is the weakest link |
| Terminal allow-stop double-check | runtime.rs:6269-6281 | Strong |
| `write_closeout` refuses terminal | runtime.rs:5746-5750 | Strong (internal CLI can't self-terminate) |

**PRD-required defences that are missing or weaker**:

- Phase-specific `next_phase` transitions (`phase = execution_package → next_phase ∈ {execution, execution_package}`, etc.) — PRD §8.2305-2307 requires, no enforcement exists.
- Required lock/blueprint fingerprints on post-lock closeouts — PRD §8.2248-2269 lists as required, schema and validator make them optional.
- `resolution_ref` structural validation — PRD §7.1850 says resolution_ref must point to a closeout or REPLAN-LOG entry; `append_contradiction` only checks non-empty string (runtime.rs:4933-4935).

### 3.3 Planning / orchestration / review pillar

**Planning writeback** (`write_planning_artifacts` at runtime.rs:1568–1614) is already honestly decomposed:

1. `prepare_planning_write_context` — read lock, compute next blueprint revision, enforce validation bar.
2. Write blueprint.
3. `supersede_omitted_planning_specs` — supersede specs active in prior blueprint but omitted now.
4. `sync_planning_specs` — write SPECs + REVIEW/NOTES/RECEIPTS.
5. `sync_planning_execution_graph` — build+write or delete the graph.
6. `refresh_planning_runtime_state` — open planning gate, refresh README.
7. `build_planning_closeout` → `append_closeout_for_active_cycle`.

**Strong PRD alignment**:

- Blueprint canonical sections (PRD §6.1215-1232) enforced via `required_blueprint_sections` + `validate_blueprint_body_contract`.
- Frontier state axes (PRD §6.1389-1392) enforced via `validate_planning_spec_state`.
- Blocking-obligation rule (PRD §6.962-963) via `decision_obligation_blocks_planning_completion`.

**Execution-package gate** — all five PRD wave safe-parallelism rules enforced in `validate_wave_safe_parallelism` (runtime.rs:9528-9654):

1. `write_paths` pairwise disjoint.
2. `write_paths` don't overlap same-wave `read_paths`.
3. `exclusive_resources` pairwise disjoint.
4. Shared schema/deploy/lockfile/global-config paths → singleton wave (via `is_singleton_wave_path` at 9497-9516).
5. Unknown risk class → singleton.

The fingerprint set (`lock_fingerprint`, `blueprint_fingerprint`, `dependency_snapshot_fingerprint`, `wave_fingerprint` per PRD §7.1569-1577) is complete and revalidated on every read.

`derive_writer_packet` **cannot widen scope**: `derive_writer_packet_scope` at runtime.rs:9440 intersects the spec's declared path scope with the package's read/write scope; `validate_writer_packet` at 2722-2728 double-checks the scope match.

**Review machine defence stack** (spec-review + mission-close):

1. **Parent-held writeback authority token** — minted in `capture_review_truth_snapshot`, verifier-only on disk via `persisted_review_truth_snapshot`. Remint refused at runtime.rs:3715-3720.
2. **Parent-loop authority token** from `begin_ralph_loop_lease` — required via `CODEX1_PARENT_LOOP_AUTHORITY_TOKEN` env for `record_review_result`, `capture_review_truth_snapshot`, `capture_review_evidence_snapshot`.
3. **`validate_parent_owned_review_writeback_identity`** (runtime.rs:4625) — rejects reviewer-like caller identities by convention (prefix heuristic, evadable by rename; the other defences cover the gap).
4. **`validate_reviewer_output_evidence_refs`** — all `reviewer-output:*` refs must resolve to on-disk artifacts with matching source-snapshot fingerprints.
5. **`validate_reviewer_outputs_follow_parent_truth_snapshot`** — every cited reviewer-output's `recorded_at` ≥ snapshot's `captured_at`, preventing a child capturing its own snapshot after writing.
6. **`validate_clean_review_lane_completion`** — required lanes (spec+code for code-producing slices) must each have a distinct reviewer-output; uniqueness enforced by `used_output_ids.insert`.
7. **Review wave contamination** — `current_review_truth_fingerprints` walks `PLANS/<mission>` + `.ralph/missions/<mission>` (excluding review subdirs) and hashes every file; any modification during the review wave is detected and flagged via `reviewer_lane_truth_mutation_detected`.

**Partial gaps in the review pillar**:

- Lane satisfaction uses **reviewer-id substring matching** (`reviewer_output_satisfies_required_lane` at runtime.rs:4558-4575). Strings like `code`, `bug`, `correctness` for the code lane; `spec`, `intent`, `proof` for the spec lane. A reviewer named `specialist-codec` matches both; uniqueness saves clean coverage from a single-reviewer attack, but two separately-named reviewers with weak coverage could technically satisfy both lanes. Stronger: require reviewer-output to declare its lane in the inbox artifact.
- Mission-close verdict uses `unresolved_blocking_gate_refs_for_source_package` (runtime.rs:7551-7570), which filters to gates evaluated against the same source package. PRD §7.1794 says "`complete` is illegal while any required gate is `open`, `failed`, or `stale`" — the code's scope is narrower. The fix should not simply broaden this to all historical gates, because superseded stale gates could deadlock close. The better shape is one canonical `required_gates_for_current_mission_truth()` derivation that computes active required gates from current lock, blueprint, active specs, package lineage, unresolved contradictions, and mission-close requirements.
- `#[cfg(test)]` bypass at runtime.rs:5619-5624 means in-crate unit tests exercise a different authority path than production. Integration tests (child CLI process) cover it, but anyone adding a new unit test for a writeback helper without explicit lease setup is silently testing the wrong code path.

---

## 4. Cross-cutting duplication and complexity

### 4.1 `runtime.rs` is a 17,766-line monolith

The module surface has already settled enough to split cleanly. Proposed decomposition — every boundary already has low cross-call surface:

| Module | Lines today | Responsibility |
| --- | --- | --- |
| `types.rs` | ~1,100 | All top-level enums + struct records (ExecutionPackage, ReviewBundle, etc.) |
| `constants.rs` | ~40 | ALLOWED_REVIEW_FINDING_CLASSES, REVIEWER_AGENT_OUTPUT_EVIDENCE_PREFIXES, REVIEW_WAVE_CONTAMINATION_EVIDENCE_PREFIXES, CODEX1_PARENT_LOOP_*_ENV, REQUIRED_MISSION_CLOSE_REVIEW_LENSES |
| `mission_bootstrap.rs` | ~300 | initialize_mission, ensure_paths_match_mission |
| `templates.rs` | ~1,200 | default_*_body + render_*; most should collapse into `templates/mission/*.md` |
| `markdown.rs` | ~310 | normalize_markdown_heading, markdown_level_two_sections, section_list_items, section_table_rows, validate_blueprint_body_contract, validate_spec_body_contract |
| `planning_writeback.rs` | ~1,400 | write_planning_artifacts and helpers |
| `execution_package.rs` | ~2,100 | compile/validate execution package, derive/validate writer packet, evaluate_execution_package_contract, wave manifest |
| `execution_graph.rs` | ~800 | build/validate execution graph, fingerprints |
| `path_scope.rs` | ~230 | PathScope normalization + overlap |
| `gates.rs` | ~480 | Gate lifecycle + index |
| `review_bundle.rs` | ~1,000 | compile/validate bundle + evidence snapshot |
| `reviewer_output.rs` | ~400 | record + validate reviewer-output inbox |
| `review_truth.rs` | ~700 | truth snapshot capture + guard bindings + lane/contamination checks |
| `review_result.rs` | ~1,000 | record_review_result + closeout building |
| `contradiction.rs` | ~400 | append_contradiction, append_replan_log, resume override |
| `selection.rs` | ~250 | open/resolve/consume selection wait |
| `resume.rs` | ~1,500 | resolve_resume, stop-hook output, child-lane reconciliation |
| `ralph_lease.rs` | ~260 | begin/pause/clear/inspect + authority verifier |
| `closeouts.rs` | ~300 | write_closeout, acknowledge_waiting_request |
| `io.rs` | ~130 | atomic_write + fsync_dir + load helpers |

Total ≈ 15,000 LOC in 18 modules, none exceeding ~2,100 LOC. The only non-trivial boundary is `gates.rs ↔ review_result.rs` (both mutate gate records); solve by making gates.rs the gate writer API and review_result.rs a consumer.

### 4.2 Snapshot / record field duplication

`MissionContractSnapshot` (mission_contract.rs:10-55) and `CloseoutRecord` (ralph.rs:73-124) share ~22 fields. Three constructors hand-copy them:

- `snapshot_from_latest_closeout` (mission_contract.rs:235-265): 26 field copies.
- `contradictory_snapshot` (:298-332): 25 field copies.
- `orphan_active_cycle_snapshot` (:336-367): 23 field copies.

Plus `apply_resume_state_override` (runtime.rs:7783-7824) mutates ~9 fields. Every new contract field requires edits in at least 3-4 places.

**Proposed fix**: extract a shared `CloseoutContractCore` with the overlapping fields, use `#[serde(flatten)]` in both wrappers.

### 4.3 `ReviewBundle` vs `ReviewEvidenceSnapshot` field duplication

ReviewEvidenceSnapshot duplicates 14 fields of ReviewBundle; `validate_review_evidence_snapshot` contains ~15 tautological "snapshot.X == bundle.X" checks. The two-file split is load-bearing (parent-held review truth vs child-readable evidence) but the field duplication is not.

**Codex1 correction:** do **not** compose the child-visible snapshot from the full `ReviewBundle` type. This is an authority boundary, and `#[serde(flatten)]` or whole-type composition would make future `ReviewBundle` fields child-visible by default. The safer simplification is an explicit `ChildReviewBundleView` that names only child-safe fields; any new bundle field then requires an intentional decision about whether it belongs in child evidence.

### 4.4 Atomic-write triplication with semantic drift

- `atomic_write_json` (ralph.rs:612-631) — bytes → temp → fsync temp → persist → fsync parent.
- `write_json` (runtime.rs:7934-7953) — identical shape.
- `atomic_write_string` (backup.rs:477-489) — **no `fsync_dir`**, violating PRD §8.2429 durable-write contract.

Fix: consolidate into one `codex1_core::atomic::write_bytes(path, &[u8])` primitive that always fsyncs parent; thin `atomic_write_json` / `atomic_write_string` wrappers.

### 4.5 `fsync_dir` duplication

- `ralph.rs:674-679` (private).
- `runtime.rs:7955-7960` (private).

Identical. Fold into the new `io.rs` module.

### 4.6 `validate_closeout` wrapper layering

- Real implementation: `validate_closeout_contract` (mission_contract.rs:94).
- Pass-through wrapper: `validate_closeout` (ralph.rs:292).
- Re-exported via `lib.rs:45`.
- Parallel half-validator: `write_closeout` at runtime.rs:5746-5792 enforces additional rules (e.g., non-empty summary) that `validate_closeout_contract` does not.

Fix: drop ralph.rs wrapper; fold write_closeout extras into a `validate_closeout_contract_strict` variant or phase-gate them into the main contract.

### 4.7 Five review-outcome resolvers over the same context

Every `record_review_result` path invokes all five:

- `review_result_verdict` (runtime.rs:4723-4755)
- `review_result_next_phase` (:4757-4792)
- `review_result_next_action` (:4794-4836)
- `review_result_reason_code` (:4838-4864)
- `review_result_continuation_prompt` (:4866-4905)

Each branches on the same `(input, context, unresolved_gates, mission_close_findings)`. A new branch in one but not the others produces drift. Fix: consolidate into one `compute_review_outcome(ctx) -> ReviewOutcomeResolution { verdict, next_phase, next_action, reason_code, continuation_prompt }`, single match on verdict × bundle_kind × passed × next_required_branch.

### 4.8 Stringly-typed protocol fields

- `reason_code: Option<String>` validated as machine token but no central registry. Every writer invents its own (`"planning_artifacts_written"`, `"execution_package_passed"`, `"review_clean"`, ...). A `ReasonCode` enum would centralize.
- `governing_revision: Option<String>` with ad-hoc shapes: `"lock:1"`, `"blueprint:3"`, `"package:<id>"`, `"spec:<id>:<rev>"`, `"mission:<id>:close"`, `"clarify:mission_state"`. Typed `GoverningRef` enum with Display would be safer.

### 4.9 Subagent-lane classification duplication

Three overlapping mechanisms for "is this a subagent lane?":

- `lane_role: Option<StopHookLaneRole>` enum (the cleanly-typed one).
- `child_lane_kind: Option<String>` free-form, checked against hand-coded strings.
- `task_path: Option<String>` with lowercase + prefix + substring checks.

Implementations in two places: `stop_hook_input_is_subagent_lane` / `_findings_only_review_lane` (internal/mod.rs:622-661), and `reviewer_identity_looks_like_child_lane` (runtime.rs:4686-4697). Different string rules; evadable by renaming a child to `specialist_codec_review_helper` which may miss all tracked prefixes.

Fix: a typed `ChildLaneKind` registered in `codex1-core::types`, normalization helper that classifies once and carries the result through downstream.

### 4.10 Template rendering is 700 lines of code instead of data

`default_mission_state_body` (runtime.rs:6588), `default_outcome_lock_body` (:6678), `default_readme_body` (:6866), `default_spec_body` (:6914), `default_spec_scope_hint` (:7032), `default_spec_review_body` (:7045), `default_spec_notes_body` (:7072), `default_receipts_readme_body` (:7085), `default_review_ledger_body` (:7098), `default_replan_log_body` (:7108), `render_mission_readme` (:6796), `render_review_ledger` (:7152), `render_spec_review` (:7293) — all are `format!` strings or conditional markdown assembly in Rust.

The `templates/mission/` tree already holds the canonical forms (used by `include_str!` in artifacts.rs tests). The runtime uses `resolve_templates_root` + `render_template` + `extract_markdown_template_body` + `render_template_body_or_fallback` (runtime.rs:7971-8091) to substitute `{{KEY}}` markers, but falls back to the Rust format strings when templates go missing.

Two different strategies for the same template tree. Fix: single-source-of-truth in `templates/mission/*.md`; runtime is just `render_template(paths, name, pairs)` + data providers for the runtime values. Target: ~150 LOC of helpers instead of ~700 LOC of inline defaults.

### 4.11 `VisibleArtifactTextKind` DSL is match arms instead of data

`artifacts.rs:75-164` is 90 lines of match over three kinds, each constructing a `VisibleArtifactTextRequirement` with `required_headings` + `required_phrases` + `section_requirements`. The shapes overlap. Every time a README gets a new section, two places change (template + requirement). Fix: move into `const` slice or YAML read once; validator stays identical.

### 4.12 Dead public API + dead fields

- **`ralph::GateStatus` and `ralph::GateEntry`** (ralph.rs:57-71, re-exported via lib.rs:41) — unused. `MissionGateStatus` is what runtime uses. `qualify.rs` has its own orthogonal `GateStatus`.
- **`WorkstreamSpecFrontmatter.spec_fingerprint: Option<Fingerprint>`** (artifacts.rs:562) — always `None`. Dead.
- **`ExecutionPackageStatus::{Draft, ReadyForGate, Superseded, Consumed}`** — never assigned; either delete/derive them from gates+closeouts or make package status the canonical lifecycle source.
- **`LockStatus::{Reopened, Superseded}`** — never assigned.
- **`_blueprint_revision` parameter** on `load_mission_close_spec_ids` / `load_descoped_mission_close_spec_ids` (runtime.rs:10294, 10321) — ignored; drift check happens elsewhere.

### 4.13 `RalphLoopLeaseMode` ≠ skill taxonomy

Skills: `clarify`, `autopilot`, `plan`, `execute`, `review-loop`, `close`. Lease modes: `AutopilotLoop`, `PlanningLoop`, `ExecutionLoop`, `ReviewLoop`. Missing: `MissionCloseLoop` (piggybacked on `ReviewLoop`), intentionally no `ClarifyLoop` (manual clarify is lease-less). Mission-close vs mid-mission review share the same mode, which is a discoverability cost.

### 4.14 Inconsistent active-cycle delete disciplines

- `append_closeout_and_rebuild_state` uses `atomic_remove_file` (ralph.rs:546-548) — rename-tombstone-fsync-delete-fsync.
- `resolve_selected_mission` uses raw `fs::remove_file` (runtime.rs:5984).
- `cleanup_transient_active_cycle` (runtime.rs:7758) is a third variant with cycle_id match but raw `remove_file`.

Fix: always use `atomic_remove_file`.

---

## 5. PRD alignment verdicts

| PRD rule | Evidence | Verdict |
| --- | --- | --- |
| `review_contract_change` is first-class trigger (§7.1602) | TriggerCode::ReviewContractChange at runtime.rs:83; mapped to Blueprint in ReplanBoundary::default; enforced via derive_package_replan_boundary | **Yes** |
| `replan_boundary` copied consistently (Spec/Package/Packet) (§7.1603) | Spec frontmatter → package → packet chain enforced + mismatches detected | **Yes** |
| Mission-close review required before complete (§7.1749-1754) | Only MissionClose bundles yield Complete (review_result_verdict:4735-4739) | **Yes** |
| "complete illegal while any required gate open/failed/stale" (§7.1794) | Verdict uses unresolved_blocking_gate_refs_for_source_package (narrower scope than PRD) | **Partial** — mission-close eligibility needs a canonical `required_gates_for_current_mission_truth()` derivation, not source-package-only or all-history checks |
| `clarify_status = ratified` on lock (§5.758-759) | initialize_mission derives lock_status from clarify_status but no reverse enforcement | **Partial** — relies on caller |
| Passed-only authorization for execution (§7.1520-1522) | derive_writer_packet bails non-Passed; validate_execution_package flags non-Passed | **Yes** |
| Parent-held token never in child context (§8) | persisted_review_truth_snapshot + persisted_ralph_loop_lease both strip token; test verifies no token in child evidence | **Yes** |
| Decision-obligation completion bar (§6.962-963) | decision_obligation_blocks_planning_completion + planning writeback + package contract | **Yes, more strict than PRD** (ignores `affects` for Major) |
| `bundle_kind = mission_close` mandatory for complete (§7.1710) | See above | **Yes** |
| Reopen rule (§5.811-818, §6.1419-1425) | ReopenLayer enum + ReplanBoundary trigger matrix + append_contradiction rejects non-local contradictions from local means | **Yes** |
| Lock reopen produces new revision (§5.14, §6.26) | No codepath writes `LockStatus::Reopened`; `LockStatus::Superseded` only via `ClarifyStatus::Superseded` mapping in `initialize_mission` (runtime.rs:1334), not a reopen workflow; no `lock_revision` bump anywhere | **No — missing** |
| ExecutionPackage lifecycle {draft, ready_for_gate, passed, failed, superseded, consumed} (§7.17) | Only Passed/Failed assigned | **Partial** — lifecycle truth needs one canonical owner; current dead variants should be removed or fully owned |
| Phase-specific next_phase transitions (§8.2305-2307) | No enforcement in validate_closeout_contract | **No — missing** |
| resolution_ref points to discharging artifact (§7.1850) | Only non-empty check in append_contradiction | **Partial** — no structural validation |
| Required fingerprint fields on post-lock closeouts (§8.2248-2269) | Schema allows null fingerprints on all closeouts | **Partial** — phase-sensitive enforcement would close |
| Stop-hook block for actionable non-terminal (§8.2219-2220) | Gates on loop-lease; without lease emits advisory only | **Partial PRD silent** — lease and mode/authority semantics are load-bearing but absent from PRD |

---

## 6. Concrete simplification opportunities

Ranked by (impact × simplicity-gain). Categories: **[S]tructural** (big mechanical wins), **[C]orrectness** (PRD gaps), **[D]uplication** (DRY), **[E]rgonomics/typing** (stringly → typed).

| # | Category | Change | Impact | Risk |
| --- | --- | --- | --- | --- |
| 0 | C | **Promote the mode/authority model to a PRD-level contract.** Define parent-loop vs discussion/interrupt vs subagent vs findings-only-reviewer modes, who may hold Ralph lease authority, who may mutate mission truth, and when stop-hook may block. | Highest (addresses the real failure mode behind repeated contaminated review/loop incidents) | Medium (requires PRD, skills, runtime, and tests to agree) |
| 1 | S | **Split `runtime.rs` into 18 modules** (see §4.1). 4–5 PRs. | Highest (reduces review/discovery cost for every future change) | Low (compilation plumbing only; all integration tests pass against public API) |
| 2 | D | **Consolidate five review-outcome resolvers** into one `compute_review_outcome(ctx) -> ReviewOutcomeResolution`. | High (drops ~180 LOC, kills drift risk) | Low (existing tests pin verdicts+phases from closeouts) |
| 3 | S | **Move ~700 lines of `default_*_body` / `render_*` into `templates/mission/*.md`**. | High (shrinks runtime.rs ~4%, single source of truth) | Low (end-to-end tests exercise full mission bootstrap) |
| 4 | C | **Remove `#[cfg(test)]` bypass** in `validate_required_parent_loop_authority_for_review_writeback` (runtime.rs:5619-5624). Provide a test helper that mounts a verifier-backed lease. | High (security-relevant function should have identical test + prod paths) | Medium (may require test updates, but the helper already exists in integration tests) |
| 5 | D | **Unify atomic writers**: one `atomic_write_bytes` primitive; `atomic_write_string` always fsyncs parent. | Medium (closes PRD durable-write contract gap for backup manifests) | Low |
| 6 | D | **Dedupe `MissionContractSnapshot` ↔ `CloseoutRecord`** via shared `CloseoutContractCore` + `#[serde(flatten)]`. | Medium-high (three snapshot constructors → one builder; ~80 LOC removed) | Low (round-trip tests catch regressions) |
| 7 | D | **Replace duplicated child snapshot fields with an explicit `ChildReviewBundleView`** instead of composing from full `ReviewBundle`. Remove tautological drift checks without making future parent-only fields child-visible by default. | Medium | Medium (one-time snapshot wire-format migration) |
| 8 | C | **Implement lock reopen**: `reopen_outcome_lock(paths, input)` that bumps `lock_revision`, sets `LockStatus::Reopened`, cascades blueprint/package gate invalidation. | High (closes the highest-severity PRD contract gap) | Medium (requires coherent invalidation cascade — the hooks exist) |
| 9 | C | **Decide the canonical source for execution-package lifecycle truth.** Prefer treating package `status` as immutable validation result and deriving superseded/consumed lifecycle from gates+closeouts, or fully invert the model so package status is canonical. Do not write both as independent mutable truths. | Medium-high (package files stop implying lifecycle semantics they do not own) | Medium (requires reader cleanup or schema simplification) |
| 10 | C | **Enforce phase-specific `next_phase` transitions** in `validate_closeout_contract` (e.g., `phase = execution_package → next_phase ∈ {execution, execution_package}`). | Medium | Low-medium |
| 11 | C | **Introduce `required_gates_for_current_mission_truth()`** and use it for mission-close legality. Derive the required gate set from lock, blueprint, active specs, package lineage, unresolved contradictions, and mission-close requirements instead of either source-package-only or all-history gate checks. | Medium-high (closes false-complete gap without stale-gate deadlocks) | Medium (tests may expose ambiguous historical gate assumptions) |
| 12 | E | **Eliminate three-meaning `spec_fingerprint`**: rename `IncludedSpecRef.spec_fingerprint → spec_content_fingerprint`, `ExecutionGraphNode.spec_fingerprint → spec_contract_fingerprint`, delete `WorkstreamSpecFrontmatter.spec_fingerprint`. | Medium (readability) | Medium (disk format rename + serde alias) |
| 13 | E | **Introduce typed `ReasonCode` and `GoverningRef`**. Centralizes protocol vocabulary; catches typos at compile time. | Medium | Medium (deserialization may need compatibility) |
| 14 | D | **Flatten `VisibleArtifactTextKind` DSL into data** (const slice or YAML). 90 lines → ~15. | Low | Trivial |
| 15 | D | **Consolidate `fsync_dir`** (ralph.rs:674 + runtime.rs:7955). | Low | Trivial |
| 16 | D | **Drop `validate_closeout` wrapper in ralph.rs**; fold write_closeout's extras into `validate_closeout_contract_strict`. | Low | Low |
| 17 | E | **Replace child-lane classification heuristics with typed `ChildLaneKind` + single matcher** (kill the three-way overlap in stop-hook and reviewer-identity paths). | Low-medium | Medium (external callers pass these fields) |
| 18 | D | **Consolidate active-cycle delete to always use `atomic_remove_file`**. | Low | Trivial |
| 19 | E | **Require reviewer-output to declare its lane** (enum `ReviewerLane`) instead of inferring via reviewer-id substring matching. | Medium (structural rather than conventional defence) | Medium (schema addition + migration) |
| 20 | C | **Strengthen `resolution_ref` validation** to require `closeout:<id>` or `replan-log:<timestamp>` shape and verify the referenced record exists. | Low-medium | Low |
| 21 | D | **Delete dead code**: `ralph::GateStatus`, `ralph::GateEntry`, `WorkstreamSpecFrontmatter.spec_fingerprint`, unused `_blueprint_revision` parameters. | Low | Trivial |
| 22 | E | **Align `RalphLoopLeaseMode` with skill taxonomy**: add `MissionCloseLoop` variant so mission-close review is distinguishable from mid-mission review. | Low | Medium |
| 23 | E | **Move `ReplanBoundary::default()` 40-line matrix into `templates/default-replan-boundary.json`**. | Low | Trivial |
| 24 | C | **Clear waiting identity in `contradictory_snapshot`** to match interrupted-cycle behavior. Same verdict (ContinueRequired) but different cleanup — latent inconsistency. | Low | Low |
| 25 | C | **Document the loop lease in the PRD**: add a section stating stop-hook block enforcement requires an active `RalphLoopLease`; without one, stop-hook emits advisory systemMessages. | High (aligns PRD to code reality — load-bearing semantics shouldn't live only in skill files) | None (documentation) |
| 26 | C | **Audit and harden global setup/init/doctor/restore UX as product surface.** Prove global setup installs the intended user-level skill/config/hook surface, project init remains explicit, backups are restorable, and doctor gives truthful repair guidance. | High (this is how Codex1 becomes usable outside this repo) | Medium (touches user-owned config and install semantics) |

---

## 7. Priority-ranked simplification roadmap

**Wave 0 (product contract first)** — do this before large refactors:

- **#0 Promote the mode/authority model to PRD-level truth.** Define parent-loop vs discussion/interrupt vs subagent vs findings-only-reviewer modes, and make the PRD, skills, runtime, and tests agree.
- **#25 Document the loop lease in the PRD**, but as part of the broader mode/authority model rather than as an isolated implementation note.
- **#22 Decide whether mission-close needs a distinct lease mode** or whether `ReviewLoop` intentionally covers both mid-mission review and mission-close review.

**Wave 1 (semantic gap closures)** — close product-safety gaps before reshaping the authority-heavy code:

- **#4 Remove `#[cfg(test)]` bypass** in parent-loop authority validation and make tests exercise the production path.
- **#8 Implement lock reopen** or explicitly descoped/defer it with honest product language.
- **#10 Enforce phase-specific `next_phase` transitions**.
- **#9 Decide execution-package lifecycle truth** instead of writing mutable lifecycle state into both package files and gates/closeouts.
- **#11 Implement `required_gates_for_current_mission_truth()`** and use it for mission-close legality.
- **#20 Strengthen `resolution_ref` validation**.
- **#24 Fix contradictory-vs-interrupted waiting identity**.

**Wave 2 (review and setup hardening)** — make the user-visible product flow and delegated review model structurally honest:

- **#19 Require reviewer-output lane declaration** so clean review depends on explicit reviewer lane truth rather than reviewer-id substring inference.
- **#17 Replace child-lane classification heuristics with typed `ChildLaneKind`**.
- **#26 Audit and harden global setup/init/doctor/restore UX** as a first-class product surface.

**Wave 3 (safe shallow refactors)** — low-risk cleanup that does not require settled authority boundaries:

- Extract only clearly orthogonal pieces of **#1 Runtime split** first: `types.rs`, `constants.rs`, `io.rs`, `markdown.rs`, and `path_scope.rs`.
- **#5 Atomic-writer unification**, including parent-directory fsync for backup writes.
- **#15 `fsync_dir` dedup**.
- **#18 Active-cycle delete consistency**.
- **#21 Delete dead code**.
- **#14 VisibleArtifactTextKind DSL → data**.

**Wave 4 (deeper structural split after contracts settle)** — split authority-heavy surfaces only once the model is explicit:

- Finish the remaining **#1 Runtime split**: planning, execution package, gates, review bundle/output/truth/result, contradiction, selection, lease, resume, closeouts, mission bootstrap, and templates.
- **#2 Review-outcome resolver consolidation**.
- **#3 Template extraction** with compile-time embedded templates where possible.
- **#6 Snapshot/CloseoutRecord dedup**.
- **#7 ChildReviewBundleView snapshot simplification**.
- **#12 Rename three-meaning `spec_fingerprint`**.
- **#13 Typed `ReasonCode` + `GoverningRef`**.
- **#23 `ReplanBoundary::default` to data file**.
- **#16 Drop validate_closeout wrapper**.

---

## 8. Rigor audit — what not to simplify

The following are intentional rigor, not ceremony. Preserve them.

- **Closeout schema validation** (mission_contract.rs:144-199). The `verdict × terminality × resume_mode` exhaustive match is the single biggest false-stop defence.
- **Sequence-gap + duplicate-id + mission-id-mixing detection** on closeouts.ndjson load.
- **fs2 advisory lock + build-then-persist order** in `append_closeout_and_rebuild_state`.
- **Interrupted-cycle override** that forces ContinueRequired + clears waiting identity when an active-cycle sidecar exists for a newer cycle.
- **Contradiction resume-override matrix** and the `HaltHardBlocked` non-terminal-until-reviewed rule.
- **Parent-held review writeback authority token** with verifier-only persistence and remint refusal.
- **Review wave contamination detection** (walks `PLANS/<mission>` + `.ralph/missions/<mission>` excluding review subdirs). Scaling cost is worth it.
- **Required reviewer-lane coverage for clean review** (distinct outputs per required lane).
- **PathScope intersection in `derive_writer_packet_scope`** (writer packets cannot widen scope).
- **All five PRD wave safe-parallelism rules** in `validate_wave_safe_parallelism`.
- **`build_execution_graph` → `validate_execution_graph_for_blueprint`** cascade that prevents the graph from drifting from the blueprint silently.
- **Defence-in-depth validation at multiple layers** (proof-matrix validated on input, re-validated inside `evaluate_execution_package_contract`; decision-obligation blockers same pattern). The duplication catches stale blueprints.
- **The ArtifactDocument<F> typed-frontmatter parse**: kind-mismatch caught at parse, not at semantic-check.
- **Selection-state's four-phase lifecycle** (open/resolve/consume/supersede/acknowledge) — looks large but solves a real resume problem.

---

## 9. What the PRD should absorb

These are load-bearing product semantics that live only in skill/code files today:

1. **The mode/authority model** is a first-class product contract, not an implementation detail. The PRD should define at least these modes: discussion/interrupt, manual clarify, parent planning loop, parent execution loop, parent review-loop, autopilot loop, subagent support lane, and findings-only reviewer lane. For each mode it should state whether Ralph may block, whether the agent may mutate mission truth, and who owns writeback.
2. **The loop lease** (`.ralph/loop-lease.json`) is the gate that decides whether stop-hook blocks. Without an active lease, actionable non-terminal missions allow clean stop with an advisory `systemMessage`. This should be explicit PRD text — readers of the PRD alone cannot predict this behaviour.
3. **Mission-close review is piggybacked on `review_loop` mode**. If `MissionCloseLoop` remains fused into `ReviewLoop`, the PRD should at least call that out; if it becomes a separate mode, the PRD mode taxonomy should follow.
4. **Mandatory reviewer lane coverage for clean verdicts on code-producing slices** (`spec` + `code`) is enforced in the machine but under-specified in the PRD.
5. **Parent-authority token via `CODEX1_PARENT_LOOP_AUTHORITY_TOKEN` env** is the transport mechanism for the lease's capability — deserves explicit PRD mention.
6. **Global setup vs project init** is product surface, not support trivia. The PRD should state that `codex1 setup` owns user-level Codex runtime setup and backup, while `codex1 init` owns current-project opt-in.
7. **This machine audit is necessary but not sufficient for product trust.** The skill loop (`clarify`, `plan`, `execute`, `review-loop`, `autopilot`, `close`) needs its own UX/quality audit before shipping.

---

## 10. Closing synthesis

The machine is fundamentally aligned with the PRD's intent. Its discipline — closeouts as the only truth, fingerprint-based drift detection, parent-held review authority, review-wave contamination checks, reviewer-output inbox that cannot self-clear gates — is real rigor, not ceremony. But the machine still has one architecture-level product gap: the authority/mode model is not yet first-class enough.

- **The 17k-line monolith** is a PRD anti-goal crystallized in code. The 18-module split is valuable, but it should not outrank finishing the authority/mode contract. Shallow utility extraction can happen early; authority-heavy review/resume/lease/gate modules should wait until the product model is settled.
- **Five PRD-level gaps** (lock reopen, execution-package lifecycle truth, phase-specific next_phase, mission-close required-gate derivation, resolution_ref validation) are partial implementations rather than missing defences. Closing them follows the machine's own patterns.
- **One security-adjacent test-production drift** (`#[cfg(test)]` bypass on parent-loop authority) is worth priority-fixing.
- **The loop lease and parent/subagent/reviewer authority model** are load-bearing and should be written into the PRD rather than continuing to live as undocumented machine features referenced by skill files.
- **~15% of runtime.rs is template assembly** that would be clearer as data. ~5% is duplicated snapshot/record copies across three constructors. ~3% is stringly-typed protocol fields that could be enums.

No full rewrite is needed, but the authority/mode model does need first-class architectural consolidation. Treat this audit as a backlog and diagnosis, not as a literal execution order. A three-month incremental refactor following the corrected waves above could reduce runtime.rs to under 10k LOC, close the partial PRD gaps, eliminate test-production drift, and make every future change shorter to review — while preserving the external skill-surface behaviour that makes Codex1 feel native.

---

*End of audit. Three Opus 4.7 agents produced the pillar analyses; this synthesis integrates their findings with a unified priority roadmap. All file_path:line_number references are against the working tree at 2026-04-18.*
