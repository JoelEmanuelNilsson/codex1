# Round 11 Meta-Review

Baseline under review: `6c33536 round-10 repairs: lock lifecycle truth`

Round 11 was not clean. Sixteen reviewer agents reported raw findings in `docs/audits/round-11/findings.md`. Eight finding-review agents reviewed the shards and accepted seven P1 findings and fifteen P2 findings. Five raw findings were merged into already-open repair families rather than kept as standalone round-11 items.

Clean-round counter: reset to 0.

## Accepted Findings

| ID | Verdict | Severity | Final title | Notes |
| --- | --- | --- | --- | --- |
| F07 | Accepted | P1 | `plan check` relocks replans that reuse historical task IDs, so new work inherits stale completion state | Fresh replans can silently skip work by aliasing old `STATE.tasks` truth. |
| F09 | Accepted | P1 | Dirty planned reviews are still advertised as ready review work even though repair is the only executable next step | New runtime readiness contradiction across `status`, `task next`, `plan waves`, `plan graph`, and `review start`. |
| F10 | Accepted | P1 | A triggered replan does not actually block stale-plan execution | `status` advertises blocked/replan while work is still executable and mutable. |
| F14 | Accepted | P1 | Superseded dirty planned reviews remain current blockers after replan and can strand the mission | Replanned/superseded dirty review truth is never retired. |
| F19 | Accepted | P1 | Restarting a planned review does not fence off late results from the previous boundary | Review restarts do not establish a new identity strong enough to reject old results. |
| F20 | Accepted | P1 | Mission-close review has no round identity, so a stale clean can still pass the terminal gate | Terminal gate can be satisfied by a stale clean because no mission-close round boundary exists. |
| F23 | Accepted | P1 | `$autopilot` cannot actually finish autonomously because the documented close handoff always asks the user again | Flagship orchestration skill stalls on the normal terminal-close path. |
| F01 | Accepted | P2 | `task next` keeps advertising `mission_close_review` after close is ready or the mission is terminal | Public next-action surface drifts from `status`/`close`. |
| F02 | Accepted | P2 | Review docs are stale for implemented `review start` / `review status` envelopes | Public docs no longer match emitted JSON. |
| F03 | Accepted | P2 | `plan graph` / `plan waves` still trust a symlinked `PLAN.yaml` outside the mission root | Remaining read-side containment gap after round-10 hardening. |
| F04 | Accepted | P2 | Packet and closeout paths still trust a symlinked `OUTCOME.md` outside the mission root | Remaining read-side containment gap on OUTCOME readers. |
| F06 | Accepted | P2 | OUTCOME `mission_id` is not checked against the actual mission directory | Outcome truth can contradict the active mission and still ratify. |
| F08 | Accepted | P2 | `plan check` accepts forbidden stored `waves:` truth in `PLAN.yaml` | Canonical plan artifact still blesses a forbidden second truth surface. |
| F13 | Accepted | P2 | `task next` tells a fresh mission to plan instead of clarify | Public next-action surface skips the clarify-before-plan gate. |
| F16 | Accepted | P2 | `close record-review --findings-file` returns the wrong public error contract | Missing findings file maps to `PROOF_MISSING` with the wrong hint. |
| F17 | Accepted | P2 | Post-terminal mission-close review results are dropped instead of being audited as `contaminated_after_terminal` | Mission-close late-output audit contract is still incomplete. |
| F18 | Accepted | P2 | `CLOSEOUT.md` can falsely claim mission-close review was clean on the first round | Final artifact loses true mission-close history after a dirty-then-clean cycle. |
| F21 | Accepted | P2 | `status` swallows explicit `--repo-root` discovery failures into the bare Ralph fallback | Explicit root misconfiguration is hidden as benign “no mission.” |
| F22 | Accepted | P2 | Public `status` docs still advertise the pre-repair ambiguity behavior | Docs still describe the old bare-ambiguity fallback. |
| F24 | Accepted | P2 | `verify-installed` can still pass even when `codex1` is unusable via `PATH` from `/tmp` | Installed-binary verification still doesn’t prove the user-visible invocation path works. |
| F25 | Accepted | P2 | `doctor` reports a non-executable `codex1` file as “on PATH” | Health probe can positively report a broken PATH shadow. |
| F26 | Accepted | P2 | `review packet` docs promise proof paths the current CLI never emits | Public docs still show the wrong proof path shape. |

## Dropped, Merged, Or Non-Standalone Findings

| ID | Final disposition | Reason |
| --- | --- | --- |
| F05 | Merge target | Same root family as the already-accepted outcome-validator issue around empty required OUTCOME sections. Fix while touching validator work; do not count separately. |
| F11 | Merge target | Same family as round-10 F11. This is another symptom of stale dirty-review handoff truth after repair, not a distinct repair line. |
| F12 | Merge target | Same family as round-10 F11. `task start` reopening a repaired target is another manifestation of stale dirty-repair targeting. |
| F15 | Merge target | Same family as round-10 F14. Stale review output after replan unlock still misses the stale-audit path. |
| F27 | Merge target | Same family as round-10 F16. Review dependency readiness still over-admits non-target `AwaitingReview` dependencies. |

## Main-Thread Agreement

The main thread accepts the finding-review verdicts.

Round 11 is not clean for three main reasons:

1. Review/replan/close boundary truth still does not retire or fence old review rounds correctly.
2. Replan/open-close readiness still leaks executable or terminal actions through inconsistent public surfaces.
3. A handful of public docs/install/status surfaces remain materially wrong enough to mislead operators or automation.

## Repair Priorities

1. Review/replan/mission-close boundary truth:
   - F14, F19, F20, plus merged F15 and F27 where they share the same state/identity logic.
2. Replan and dirty-review readiness contradictions:
   - F09, F10, and the remaining current-round part of the round-10 F11 family.
3. Remaining mission-truth containment and outcome truth correctness:
   - F03, F04, F06, and F08.
4. Public next-action / install / docs correctness:
   - F01, F02, F13, F16, F17, F18, F21, F22, F23, F24, F25, F26.
