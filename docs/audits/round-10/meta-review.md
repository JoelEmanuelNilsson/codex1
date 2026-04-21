# Round 10 Meta-Review

Baseline under review: `9169e98 round-9 repairs: harden lifecycle concurrency`

Round 10 was not clean. Sixteen reviewers reported raw findings in `docs/audits/round-10/findings.md`. Eight finding-review agents independently reviewed the shards and accepted two P1 findings and eleven P2 findings. Four raw test-adequacy findings were downgraded to non-standalone checklist items.

Clean-round counter: reset to 0.

## Accepted Findings

| ID | Verdict | Severity | Final Title | Notes |
| --- | --- | --- | --- | --- |
| F10 | Accepted | P1 | `close complete` can publish stale or rejected `CLOSEOUT.md` before the terminal mutation wins | Merge of R10-09/R10-10. Terminal artifact/state transaction bug and unfenced concurrent close idempotency failure. |
| F11 | Accepted | P1 | Dirty-review repair keeps routing to `repair` after the target was repaired and is ready for re-review | Merge of R10-12/R10-16. Continuation of round-9 F13; status handoff still loops after repair finish. |
| F01 | Accepted | P2 | `outcome ratify` can fail after ratifying state | Symlink/write-safety preflight occurs after state mutation. |
| F02 | Accepted | P2 | `task next` collapses ready tasks from multiple topological waves into one `run_wave` | Current-wave derivation drift between `task next`, `status`, and `plan waves`. |
| F03 | Accepted | P2 | `status.stop.allow` depends on close blockers despite stop contract | Regression from proof-aware close readiness repair; `stop.allow` must remain verdict/loop based. |
| F08 | Accepted | P2 | README and CLI reference give `task finish --proof` paths that fail | Docs manual happy path uses repo-relative proof paths where CLI expects mission-relative paths. |
| F09 | Accepted | P2 | CLI reference and README omit live mission-close and loop subcommands needed for the documented flow | Docs omit `loop activate` and `close record-review`; README flow cannot terminal-close as written. |
| F12 | Accepted | P2 | Concurrent loop transitions classify before lock and can resurrect a deactivated loop | Wet loop transitions apply stale pre-lock target state. |
| F13 | Accepted | P2 | Concurrent `task finish` can double-commit and overwrite proof truth | Missing locked status revalidation/idempotency for finish. |
| F14 | Accepted | P2 | Review record racing with replan returns `PLAN_INVALID` and drops stale audit event | Locked path checks `plan.locked` before stale/superseded classification/audit. |
| F15 | Accepted | P2 | Top-level mission truth files are trusted through symlinks outside `PLANS/<mission>` | Read-side containment gap for `STATE.json`, `PLAN.yaml`, and `OUTCOME.md`. |
| F16 | Accepted | P2 | Review commands can complete a review before the review task's own DAG dependencies are ready | Review command parser ignores `depends_on`; only target states are checked. |
| F17 | Accepted | P2 | Harmless state mutations while a review is open make valid findings audit-only | `late_same_boundary` keys off global revision rather than review-relevant boundary changes. |

## Dropped Or Non-Standalone Findings

| ID | Final Disposition | Reason |
| --- | --- | --- |
| F04 | Non-standalone checklist | Runtime behavior is already correct: stale `review start --expect-revision` wins over malformed `PLAN.yaml`. Optional exact regression can be added. |
| F05 | Non-standalone checklist | Close-review clean-before-stale-dirty runtime invariant appears protected by locked revalidation. Optional stress regression remains useful. |
| F06 | Non-standalone checklist | Replan orphan review-task close path works at runtime; current e2e test still masks it with manual mutation and can be strengthened. |
| F07 | Non-standalone checklist | Status live-work/superseded dependency behavior appears correct; only missing exact regression coverage. |

## Duplicate And Merge Decisions

- F10 merges close lifecycle and state/concurrency reports into one P1 close-complete transaction finding.
- F11 merges loop/orchestration and current-diff reports into one P1 dirty-review repair handoff finding.
- F13 merges task lifecycle and state/concurrency reports into one P2 `task finish` concurrency finding.
- F14 merges task lifecycle and current-diff reports into one P2 review-record/replan stale audit finding.
- F08 and F09 are both docs P2s but remain separate: F08 is proof path correctness, F09 is missing command/manual mission-close flow.
- F15 is related to prior write-side symlink containment repairs, but is distinct: it is read-side truth poisoning.
- F17 is related to the accepted late-record audit-only contract, but is distinct: it over-applies lateness to unrelated state mutations while the same review boundary remains current.

## Main-Thread Agreement

The main thread accepts the finding-review verdicts. The accepted set is concrete, reproducible, and aligned with prior audit policy:

- P1 findings block core lifecycle correctness or terminal artifact truth.
- P2 findings break documented CLI contracts, concurrency/idempotency guarantees, or mission-truth containment.
- Pure coverage gaps without a current runtime failure are not counted as standalone P0/P1/P2 findings, but can be addressed opportunistically when touching nearby tests.

