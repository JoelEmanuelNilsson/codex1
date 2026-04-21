# Round 6 Meta-Review

## Verdict

The round-6 report is mostly useful, but it over-counts P1s. Accept 5 of 7 P1s, drop 2 P1s as stale/false positive, keep most P2s, and downgrade P2-5 to doc-cleanup/P3. Add one missing P2 documentation issue around the mutation protocol.

Adjusted count: P0 0, P1 5, P2 8 if the missing docs issue is added.

## Finding Review

| Finding | Meta-review |
| --- | --- |
| P1-1 close complete state-before-CLOSEOUT | Valid P1. Terminal state can become unrecoverable without `CLOSEOUT.md`. Also fix its `--expect-revision` check order while touching it. |
| P1-2 close dirty review state-before-findings | Valid P1. A post-state artifact write failure can double-count dirty mission-close reviews on retry and trigger replan early. |
| P1-3 mission ids escape `PLANS/` | Valid P1. Raw mission ids are joined as path components. This is a real path traversal write issue. |
| P1-4 spec paths read outside mission | Valid P1. `plan check`, `task packet`, and `review packet` trust task spec paths enough to expose arbitrary readable files. |
| P1-5 autopilot escalates recoverable blocked states | Drop. Current `.codex/skills/autopilot/SKILL.md` dispatches by `next_action.kind` and has explicit `repair` and `replan` rows; the reference table also covers `blocked + repair/replan`. The `blocked` row means `next_action.kind = blocked`, not every blocked verdict. |
| P1-6 close commands hide stale-writer conflicts | Valid P1. `close complete` and `close record-review` perform readiness/terminal gates before `check_expected_revision`, contradicting strict stale-writer semantics. |
| P1-7 handoff omits live loop/review verbs | Drop as stale/false positive. `docs/codex1-rebuild-handoff/02-cli-contract.md` already lists both `codex1 loop activate` and `codex1 close record-review`, and includes explanatory prose for both. |
| P2-1 status reports unsafe waves as parallel-safe | Valid P2. `status` ignores `exclusive_resources` / `unknown_side_effects` while `plan waves` computes blockers. |
| P2-2 task next wave id order-dependent | Valid P2. `task next` computes wave depth in plan order, unlike the fixed-point/topological derivations elsewhere. |
| P2-3 no-mission status has wrong `foundation_only` | Valid P2, small contract drift. |
| P2-4 terminal planned-review records dropped | Valid P2. The late-output contract says terminal-contaminated review records are audited; current code returns before appending an event. |
| P2-5 execute skill hides real replan surface | Downgrade to P3 / docs cleanup. The sentence saying `task next` cannot surface `replan` is stale, but the workflow reads `status` as the source of truth and explicitly handles `replan`, so the stated P2 impact is overstated. |
| P2-6 mission-close clean does not reset dirty counter | Valid P2. Existing tests encode the current behavior, but the "consecutive dirty" semantics and contract favor resetting `__mission_close__` on clean. Impact is mostly telemetry unless future reopen/retry paths rely on the stale count. |
| P2-7 task next omits unsafe-wave blockers | Valid P2. Same root as P2-1, but a separate public surface; fix together via shared wave-safety derivation. |
| P2-8 stale NOT_IMPLEMENTED docs | Valid P2. README and CLI reference still describe implemented commands as not implemented. |

Missing issue: `docs/cli-contract-schemas.md` still describes `state::mutate` as writing `STATE.json` before appending `EVENTS.jsonl`, while code and round-1 decisions intentionally require EVENTS-before-STATE for recoverable crash behavior. Add this as a P2 or fold it into P2-8's docs refresh.

## Repair Order

1. Fix path containment first: P1-3 and P1-4. Add a mission-id validator and a reusable "path must stay under mission dir" helper, then guard `plan check`, `task packet`, and `review packet`.
2. Fix close/review atomicity and revision ordering together: P1-6, P1-1, P1-2, then P2-6. The close surfaces are clustered and should get one coherent consistency pass.
3. Unify wave derivation: P2-1, P2-2, and P2-7. Prefer one shared parser/derivation path for wave id, ready wave, `parallel_safe`, and blockers.
4. Patch audit/status contract drift: P2-4 and P2-3.
5. Refresh docs and skills last: P2-8, the missing mutation-protocol docs issue, and downgraded P2-5.

## Notes

P1-1 and P1-2 share the same design smell: state commits are exposed before required companion artifacts are durable. The exact fix can differ, but the invariant should be "no successful state transition leaves required mission truth unrepairable."

P2-1 and P2-7 should not be fixed independently with copied logic. Copying the `plan waves` blocker rules into both surfaces would likely create the next drift. A small shared helper is warranted here.

P1-5 and P1-7 should be removed from the round-6 findings before repair planning, or they will waste time and distort priority.
