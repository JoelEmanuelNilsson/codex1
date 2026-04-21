# Round 8 Repair Plan

Date: 2026-04-21

This plan follows `docs/audits/round-8/meta-review.md`.

## Accepted Findings

### P1

1. Write-side symlink containment escape for mission/review artifacts.
2. `review start` can erase active dirty review truth.
3. Late dirty review records can become current blockers.
4. Stale mission-close dirty records can reopen after clean.
5. Concurrent mission-close stale writers can overwrite committed findings artifacts.
6. `$execute` rejects valid status/autopilot handoffs.

### P2

1. OUTCOME validator misses required `definitions` and `resolved_questions`.
2. Review tasks can omit reviewed targets from `depends_on`.
3. `plan waves` can report a downstream ready wave while an upstream dependency is in progress.
4. `review start` still masks stale revision conflicts behind plan parsing.
5. Superseded live-DAG projection disagrees across `plan waves`, `status`, and `task next`.
6. Ralph fails open on ambiguous multi-mission status errors.
7. `verify-installed` can pass without verifying the concrete installed binary and `/tmp doctor` smoke.
8. Close can complete after required proof files are deleted.
9. Replanning reviewed work can strand orphan review tasks.
10. Review packet drops recorded absolute proof paths.
11. Superseded-root downstream live-DAG regression coverage is missing as part of the live-DAG repair.

## Implementation Plan

1. Path containment.
   - Add write-side containment helpers in `core/paths.rs`.
   - Reject symlinked existing mission directories in `init`.
   - Validate mission root and artifact parents such as `reviews/` before CLI-owned writes.
   - Tests: symlinked `PLANS/<mission>` on init, symlinked `reviews/` on review record, no outside write.

2. Outcome validator.
   - Require `definitions` to exist as a YAML mapping.
   - Require `resolved_questions` to exist as a YAML sequence; validate non-empty entries have `question` and `answer`.
   - Tests: missing fields and wrong types fail `outcome check` and `outcome ratify`.

3. Review DAG and live-DAG/wave behavior.
   - In `plan check`, require review task `depends_on` includes all `review_target.tasks`.
   - Prevent review tasks from being started or surfaced as normal `task next` / `status` work.
   - Centralize or align live-DAG rules so `plan waves`, `status`, and `task next` agree on superseded dependencies and in-progress upstream dependencies.
   - Tests: review target dependency omission rejected; in-progress upstream does not advance `current_ready_wave`; superseded root/downstream case has consistent behavior.

4. Planned review truth and stale revision.
   - In `review start`, check `--expect-revision` immediately after state load and before plan parsing.
   - Prevent `review start` from overwriting an accepted dirty review.
   - Make non-current review record categories audit-only and avoid replacing `state.reviews`.
   - Tests: stale review-start beats malformed plan; dirty review remains blocking after attempted restart; late dirty record does not alter readiness/current truth.

5. Replan / orphan review lifecycle.
   - Allow superseding DAG ids that are present in the locked plan even if absent from `state.tasks`, or automatically supersede obsolete review tasks whose target set is superseded.
   - Tests: `T2 awaiting_review` + `T3 review T2` + replan supersedes T2 does not strand close.

6. Close-review concurrency.
   - Revalidate mission-close review recordability inside the locked state mutation.
   - Ensure stale/rejected writers cannot write or overwrite the committed `mission-close-<revision>.md` artifact.
   - Tests: clean-before-stale-dirty cannot reopen; concurrent `--expect-revision` dirty writers leave artifact content from the successful writer only.

7. Proof artifacts and packets.
   - Make review packets use recorded `TaskRecord.proof_path`, preserving absolute paths and resolving mission-relative paths, with conventional fallback for legacy states.
   - Make close readiness validate every completed non-superseded task has a readable proof artifact.
   - Tests: absolute proof appears in review packet; deleted proof blocks `close check` / `close complete`.

8. Ralph, skills, Makefile.
   - Make Ralph fail closed or become mission-aware for ambiguous multi-mission status errors.
   - Update `$execute` to activate inactive loops for executable/repair handoffs and allow `blocked + repair`.
   - Bind `verify-installed` to `$(INSTALL_DIR)/codex1` and run `doctor`, `init`, and `status` from the `/tmp` smoke directory. Quote install paths while touching the Makefile.
   - Tests: ambiguous multi-mission hook behavior; skill prose checks if existing suite supports them; Makefile smoke verified through `make verify-contract`.

## Risks

- The close-review artifact fix crosses state locking and filesystem durability. Keep the change minimal and add direct concurrency tests.
- The live-DAG repair touches three public projection surfaces. Prefer shared helpers or identical predicates to avoid a fresh projection drift.
- The review truth repair must preserve audit events for late/stale outputs while preventing them from changing current mission truth.
- Write-side symlink containment should avoid rejecting normal existing mission directories, but must reject symlinked mission roots and symlinked artifact parent directories before writes.

## Verification

Run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
make verify-contract
```

After verification, start another full 16-reviewer round. The clean-round counter remains 0 until a full round produces no accepted P0/P1/P2 findings.

