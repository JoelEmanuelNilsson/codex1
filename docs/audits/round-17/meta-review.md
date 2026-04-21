# Round 17 Finding Review

Date: 2026-04-22

Baseline reviewed: `d4111e1f0b79513b6ad4a43eb14b440843c1ea40`

Round 17 used four finding-review agents on the deduplicated finding set from `findings.md`. Each finding-review agent used `gpt-5.4` with high reasoning and rechecked evidence against the rebuild handoff as the primary intended-state source.

## Decisions

| Finding | Decision | Final severity | Rationale |
| --- | --- | ---: | --- |
| F01 | ACCEPT | P1 | Mission-close dirty review can enter `review_state=open`, but follow-up clean review is rejected as not ready. This breaks the documented close-review loop and leaves the mission unable to complete. |
| F02 | ACCEPT | P2 | Closeout history still reads `reviews/` as an ordinary directory, so a symlinked reviews directory can poison rendered mission-close history with external files. |
| F03 | ACCEPT | P2 | `outcome ratify` can still publish `OUTCOME.md` before later event/state failure, leaving artifact truth ahead of state truth. |
| F04 | ACCEPT | P1 | Relock can omit non-terminal tasks from the new DAG, after which status can be blocked while action surfaces still advertise new work and later supersede repair may be impossible. |
| F05 | ACCEPT | P1 | After a dirty review target is repaired, action surfaces can advance to re-review while the top-level verdict remains globally blocked by the live review task itself. |
| F06 | ACCEPT | P2 | Review readiness admits non-target dependencies in `AwaitingReview`; only actual review targets should be allowed to be pending review at the review boundary. |
| F07 | ACCEPT | P1 | Planned dirty review can commit state/event truth before the findings artifact is durable, leaving state pointing at a missing review artifact. |
| F08 | ACCEPT | P1 | Restarted planned-review boundaries still lack a durable boundary token, so stale outputs from an earlier boundary can be accepted as current. |
| F09 | ACCEPT | P1 | Replan validation now over-rejects append-style replacement plans that keep completed prerequisite tasks, blocking a valid repair shape. |
| F10 | ACCEPT | P1 | Mission-close dirty artifacts are named from a pre-lock revision estimate, so concurrent mutation can make the success envelope point at a filename that was never written. |
| F11 | MERGE | - | Artifact transaction test gaps are covered by F07/F10/F15 repairs and regression tests. |
| F12 | MERGE | - | Orphan-task close/readiness coverage belongs with F04. |
| F13 | ACCEPT | P2 | Stale-review audit payload coverage is too weak; stale audit events should preserve category and target context. |
| F14 | ACCEPT | P2 | Active invalid-state stop behavior is important enough to pin with direct coverage while touching status/readiness. |
| F15 | ACCEPT | P1 | Mission-close dirty review can commit state/event truth before its findings artifact is durable, same transaction family as F07 but on close lifecycle. |

## Consolidated Repair Themes

- Close lifecycle: restore documented open-to-clean mission-close review flow while preserving duplicate-dirty protections; harden closeout history against symlinked review directories; compute close dirty artifact names from committed revision truth.
- Artifact/state ordering: planned review and close dirty findings must not advance state if the corresponding findings artifact cannot be written; tests should cover artifact write failures, not only preflight/event failures.
- Replan/readiness consistency: prevent stranded omitted non-terminal tasks or make them explicitly supersedable; do not advertise runnable work while orphan blockers exist; allow completed prerequisites to remain in append-style replacement plans.
- Review boundary/readiness: make review dependencies target-aware and prevent stale prior-boundary review outputs from being accepted after a restart.
- Coverage/documentation: add tight regression tests for stale audit payload context, active invalid-state stop projection, and the accepted edge cases above.

## Rejected Or Non-Standalone Items

No finding-review agent rejected a round 17 candidate outright. F11 and F12 are not standalone product bugs; they are test adequacy items that merge into accepted repair families.
