# Round 12 Meta-Review

Baseline under review: `9421cf8 round-11 repairs: harden plan/outcome contracts`

Round 12 was not clean. Sixteen reviewer agents reported raw findings in `docs/audits/round-12/findings.md`. Eight finding-review shards reviewed those candidates. Six new findings were accepted this round: two P1s and four P2s. The remaining candidates were either confirmed as continuations of already-accepted round-10/round-11 families or folded into already-open docs/contract repairs.

Clean-round counter: reset to 0.

## Accepted Findings

| ID | Verdict | Severity | Final title | Notes |
| --- | --- | --- | --- | --- |
| F04 | Accepted | P2 | `outcome check` / `outcome ratify` still bless malformed OUTCOME field domains and non-string junk entries | The empty-list repair from round 11 landed, but the remaining type/domain hole is still real. |
| F05 | Accepted | P1 | `plan check` lets a changed locked plan replace the live DAG without a replan | A locked hash change can orphan unsuperseded in-flight work from readiness and close gating. |
| F07 | Accepted | P2 | `plan check` still does not verify `PLAN.yaml mission_id` against the active mission | Top-level plan truth can still contradict the mission being locked. |
| F14 | Accepted | P1 | Side-effectful review/close mutations can publish artifacts before the event/state commit succeeds | `review record`, `close record-review`, and `close complete` can expose canonical artifacts even when the mutation fails. |
| F15 | Accepted | P2 | `plan scaffold` records success and bumps revision before confirming `PLAN.yaml` is writable | Audit/state can claim scaffold success even when the scaffold file was never published. |
| F16 | Accepted | P2 | Bare `status` incorrectly degrades an in-repo zero-candidate `PLANS/` tree into the Ralph `foundation_only` fallback | The bare no-mission fallback still swallows the empty-in-repo case that should error. |
| F17 | Accepted | P2 | `verify-installed` breaks the documented `INSTALL_DIR=<path>` flow when the install dir is relative | The `/tmp` verification step rebases relative install dirs and fails the advertised workflow. |
| F22 | Accepted | P2 | `status` and `close check` can announce terminal readiness even when `close complete` is guaranteed to fail on `CLOSEOUT.md` | Close readiness still omits a stable closeout-path blocker. |

## Dropped, Merged, Or Non-Standalone Findings

| ID | Final disposition | Reason |
| --- | --- | --- |
| F01 | Merge target | Same repair family as round-11 F01. `task next` close readiness still needs to use the shared close-readiness truth. |
| F02 | Merge target | Same repair family as round-11 F16. Missing mission-close findings files still use the wrong public contract. |
| F03 | Merge target | Same repair family as round-11 F17. Terminal mission-close late results are still dropped instead of audited. |
| F06 | Merge target | Same repair family as round-11 F07. Historical task-id reuse still leaks through `state.reviews`. |
| F08 | Merge target | Same repair family as round-11 F09. Dirty planned reviews are still over-advertised before repair. |
| F09 | Merge target | Same repair family as the round-10 F11 lineage. Post-repair `status.verdict` still blocks after repair even when re-review is ready. |
| F10 | Merge target | Same repair family as round-11 F14. Superseded dirty review truth still blocks rebuilt missions. |
| F11 | Merge target | Same repair family as round-11 F10. `task start` still bypasses the replan gate. |
| F12 | Merge target | Same repair family as round-10 F14 / round-11 merged F15. Late review results during unlocked replan still miss the stale-audit path. |
| F13 | Merge target | Same repair family as round-11 F19. Review restart still lacks a hard boundary fence. |
| F18 | Merge target | Same repair family as round-11 F18. Mission-close dirty-then-clean history still lies in `CLOSEOUT.md`. |
| F19 | Merge target | Same repair family as round-11 F02. `review status` docs are still stale. |
| F20 | Merge target | Same repair family as round-11 F16. `close record-review` docs still drift on both success and missing-file behavior. |
| F21 | Merge target | Same repair family as round-11 F20. Passed mission-close review surviving replan is another symptom of missing terminal boundary identity. |

## Main-Thread Agreement

The main thread accepts the finding-review verdicts.

Round 12 is not clean for three main reasons:

1. Plan truth is still too permissive in two places: a locked plan can still be replaced in place, and `PLAN.yaml` top-level mission identity still is not verified.
2. Artifact/state publication still is not fully transactional: close/review/scaffold flows can claim success or publish canonical files before the state/event commit succeeds.
3. A few important contract surfaces are still misleading operators and automation:
   - bare `status` still swallows an empty in-repo `PLANS/` tree,
   - relative `INSTALL_DIR` still breaks the documented verify flow,
   - close readiness still overstates when terminal close is actually executable.

## Repair Priorities

1. Preserve mission truth integrity under lock:
   - F05, F14, F15.
2. Tighten top-level plan/outcome contract enforcement:
   - F04, F07.
3. Fix remaining operator-facing readiness/install/status drift:
   - F16, F17, F22.
4. While touching those codepaths, fold in the already-open round-10/round-11 merge families that share the same files:
   - round-11 F01, F07, F09, F10, F14, F16, F17, F18, F19, F20
   - round-10 F11, F14.
