# Round 9 Meta-Review

## Verdict

Round 9 is not clean. After finding review:

- P0: 0
- P1: 4
- P2: 8
- Test checklist: 1 non-standalone item

The clean-round counter remains 0.

## Finding Review

| Finding | Meta-review verdict |
| --- | --- |
| F01 status close-ready disagrees with proof-aware close check | Accepted P2. Public projection disagreement; status must use the same path-aware readiness/blocker signal or suppress close-ready when close check would block. |
| F02 `EVENTS.jsonl` / `STATE.json.lock` symlink sidecar escape | Accepted P1. File-level mission sidecar writes can still escape `PLANS/<mission>`. |
| F03 empty OUTCOME required fields ratify | Accepted P2. Incomplete round-8 required-field repair; require non-empty `definitions` and `resolved_questions`. |
| F04 `plan check` locks before OUTCOME ratification | Accepted P2. Ratified-destination gate bypass; `plan check` must return `OUTCOME_NOT_RATIFIED`. |
| F05 plan relocks live work depending on superseded task | Accepted P2. Remaining live-DAG/supersession validation gap. |
| F06 late/rejected planned-review writers overwrite current findings artifact | Accepted P1. Current review truth can point at an artifact overwritten by late or rejected output. |
| F07 stale pre-lock category reported/audited as accepted-current | Accepted P2. Related to F06; response/event must reflect locked classification. |
| F08 concurrent `task start` not idempotent under lock | Accepted P2. Duplicate start events/revisions violate idempotency contract. |
| F09 `close complete` terminalizes before unwritable `CLOSEOUT.md` failure | Accepted P2. Historical P1 class is partially mitigated by missing-closeout recovery; still must fail before terminal state on known unwritable target. |
| F10 jq Ralph ambiguous-mission branch fails open | Accepted P2. Merge with F11 into one Ralph ambiguous fail-closed repair. |
| F11 no-jq Ralph ambiguous-mission branch fails open / exits non-blocking | Accepted P2. Merge with F10; add jq and no-jq tests. |
| F12 round-8 coverage gaps | Not standalone. Convert into regression-test checklist attached to accepted findings and prior repairs. |
| F13 repair handoff targets cannot be started | Accepted P1. Core dirty-review repair loop stalls because status advertises `repair` targets still in `AwaitingReview` and `task start` refuses them. |

## Accepted Repair Set

### P1

1. File-level symlink containment for `EVENTS.jsonl` and `STATE.json.lock`.
2. Planned-review findings artifact transaction: late/rejected writers must not overwrite current artifact.
3. Repair handoff targets must be startable and complete a repair/re-review loop.
4. Current planned-review artifact truth must remain aligned with `STATE.json`.

### P2

1. Status/close-check proof readiness agreement.
2. Empty `definitions` / `resolved_questions` rejected by OUTCOME validation.
3. `plan check` requires ratified outcome.
4. Live DAG rejects live tasks depending on superseded tasks and projections agree.
5. Review-record response/event category uses locked classification.
6. Concurrent `task start` is truly idempotent under lock.
7. `close complete` must not terminalize before discovering `CLOSEOUT.md` target is unwritable.
8. Ralph ambiguous mission errors fail closed in both jq and no-jq parsing paths.

## Repair Order

1. Sidecar containment.
2. Planned-review record transaction and category reporting.
3. Repair handoff startability.
4. Close/status proof readiness and closeout preflight.
5. Outcome and plan gates.
6. Live-DAG/supersession validation.
7. Task-start idempotency.
8. Ralph hook parsing.
9. Regression coverage sweep for F12.

