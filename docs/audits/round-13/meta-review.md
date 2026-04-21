# Round 13 Meta-Review

Baseline under review: `f9e1a1a round-12 repairs: harden plan and close truth`

Round 13 was not clean. Sixteen reviewer agents reported raw findings in `docs/audits/round-13/findings.md`. Eight finding-review shards reviewed those candidates. Eight findings were accepted this round: two P1s and six P2s. Most of the remaining candidates were confirmed as continuations of already-accepted round-10/round-11/round-12 families rather than new defect lines.

Clean-round counter: reset to 0.

## Accepted Findings

| ID | Verdict | Severity | Final title | Notes |
| --- | --- | --- | --- | --- |
| F01 | Accepted | P1 | `outcome ratify` can re-ratify a changed `OUTCOME.md` after plan lock and silently change active worker/reviewer instructions without a replan | Locked missions can currently change clarified destination truth underneath active execution. |
| F02 | Accepted | P1 | `plan check` can downgrade a recorded hard-planning mission to `light` and bypass the hard-evidence lock gate | The only enforced hard-plan safety gate can be bypassed at lock time. |
| F03 | Accepted | P2 | `plan check` accepts unknown `review_profiles`, so invalid review tasks lock and later emit unusable profiles to `$review-loop` | Plan lock still blesses malformed review orchestration data. |
| F04 | Accepted | P2 | `close complete` is not actually idempotent per the published contract | Runtime recovery behavior and docs still disagree on the second-call contract. |
| F05 | Accepted | P2 | `review record` docs still advertise `REPLAN_REQUIRED`, but the threshold path succeeds with `replan_triggered: true` | Public contract still says the wrong thing on the dirty-threshold path. |
| F07 | Accepted | P2 | Proof receipts can escape `PLANS/<mission>` through a symlinked mission-local `PROOF.md` | Relative proof receipts are still not mission-contained. |
| F18 | Accepted | P2 | `replan record` mutates terminal missions and leaves publicly contradictory terminal state | Terminal missions are still mutable through the replan surface. |
| F21 | Accepted | P2 | The published custom-`INSTALL_DIR` verification recipe can fail or verify the wrong binary from `/tmp` | Docs still misdescribe the supported custom-install verification flow. |

## Dropped, Merged, Or Non-Standalone Findings

| ID | Final disposition | Reason |
| --- | --- | --- |
| F06 | Merge target | Same docs/runtime family as round-12 F16. `status` empty-`PLANS/` discovery docs still need to match the repaired behavior. |
| F08 | Merge target | Same artifact-publication family as round-12 F14. `close complete` still exposes `CLOSEOUT.md` before commit under more failure shapes. |
| F09 | Merge target | Same dirty-review readiness family as round-11 F09 / round-12 F08. |
| F10 | Merge target | Same replan-gate/readiness family as round-11 F10. |
| F11 | Merge target | Same superseded-DAG projection family as earlier accepted wave/graph findings. |
| F12 | Merge target | Same review-boundary fencing family as round-11 F19. |
| F13 | Merge target | Same stale-after-replan audit family as round-10 F14 / round-11 F15. |
| F14 | Merge target | Same post-repair reopen family as round-10 F11. |
| F15 | Merge target | Same superseded dirty-review blocker family as round-11 F14. |
| F16 | Merge target | Same non-target review-dependency family as round-10 F16 / round-11 merged F27. |
| F17 | Merge target | Same mission-close boundary identity family as round-11 F20 / round-12 F21. |
| F19 | Merge target | Same close-readiness docs family as round-12 F22. |
| F20 | Dropped false positive | The claimed `make verify-contract` flake on missing `target/debug/codex1` was not verified by finding review as written. |
| F22 | Merge target | Same already-reviewed `replan record` docs scalar-vs-array drift family as round-8 F18; not a new round-13 product issue. |
| F23 | Merge target | Same close-readiness overstatement family as round-12 F22. |
| F24 | Merge target | Same scaffold event-before-publish family as round-12 F15. |

## Main-Thread Agreement

The main thread accepts the finding-review verdicts.

Round 13 is not clean for four main reasons:

1. Mission-root truth is still not fully frozen once the plan is locked:
   - `OUTCOME.md` can still be re-ratified after lock,
   - hard planning level can still be silently downgraded,
   - invalid review profiles can still be locked into the plan.
2. Terminal and proof trust are still too permissive:
   - `replan record` can still mutate terminal missions,
   - mission-local proof receipts can still escape through symlinks.
3. Several long-lived lifecycle families are still open even though they were merged rather than counted again:
   - dirty-review readiness drift,
   - restarted review-boundary fencing,
   - stale-after-replan audit loss,
   - mission-close boundary truth surviving replan.
4. A few docs/contract surfaces are still stale enough to mislead operators:
   - `close complete` idempotency wording,
   - `review record` threshold docs,
   - custom `INSTALL_DIR` verification recipe.

## Repair Priorities

1. Freeze mission-truth changes after lock:
   - F01, F02, F03.
2. Tighten terminal/proof safety:
   - F07, F18.
3. Correct the remaining public docs/contract drift tied to those flows:
   - F04, F05, F21.
4. While touching those files, fold in the already-open merge families they overlap with:
   - round-12 F14, F15, F16, F22,
   - round-11 F09, F10, F14, F19, F20,
   - round-10 F11, F14, F16.
