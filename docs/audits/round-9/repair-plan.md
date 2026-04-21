# Round 9 Repair Plan

Date: 2026-04-21

This plan follows `docs/audits/round-9/meta-review.md`.

## Accepted Repairs

1. Reject symlinked mission sidecar files (`EVENTS.jsonl`, `STATE.json.lock`) before opening/writing.
2. Ensure planned-review findings artifacts are published only by the accepted-current winning transaction.
3. Return locked review-record classification in response/event payloads.
4. Make dirty-review repair targets startable from `AwaitingReview`.
5. Make `status` agree with proof-aware `close check`.
6. Preflight `CLOSEOUT.md` writability before terminal state mutation.
7. Reject empty `definitions` and empty `resolved_questions`.
8. Reject `plan check` before outcome ratification.
9. Reject live non-review tasks depending on superseded tasks.
10. Make concurrent `task start` idempotent under the state lock.
11. Fix Ralph ambiguous-mission parsing in jq and fallback branches.
12. Add regression tests for the round-9 issues and round-8 coverage checklist.

## Implementation Notes

- Planned-review artifact repair should not broaden into a new artifact system. The minimal invariant is: only the accepted-current record writes `reviews/<id>.md`; late/stale/terminal records only append audit events or use separate non-current paths if needed.
- For `task start` repair mode, allow `AwaitingReview -> InProgress` only when the task is a dirty accepted-current repair target and the plan is locked with no replan trigger.
- For `status`, either pass mission paths into the projection or compute proof blockers before advertising close readiness.
- For sidecar symlink defense, reject existing symlink target paths before opening. This is not perfect against all TOCTOU attacks, but it closes the reproduced product bug and matches the current containment posture.

## Verification

Run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
make verify-contract
```

Then commit the repair pass and start another full 16-reviewer round. Clean-round counter remains 0 until a full round has no accepted P0/P1/P2 findings.

