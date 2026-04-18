---
artifact: mission-state
mission_id: review-loop-delegated-review-only
root_mission_id: review-loop-delegated-review-only
parent_mission_id: null
version: 1
clarify_status: ratified
slug: review-loop-delegated-review-only
current_lock_revision: 1
reopened_from_lock_revision: null
---
# Mission State

## Current Objective

Clarify and lock the product correction that `$review-loop` must never use parent self-review. Review judgment belongs to reviewer agent roles; parent owns orchestration and writeback.

## Provenance

- User-stated: parent is never to do review.
- Repo-grounded: current docs imply parent orchestration but do not explicitly forbid local parent review.
- Codex synthesis: lock is ready because the desired authority split is unambiguous.
