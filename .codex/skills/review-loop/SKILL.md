---
name: review-loop
description: Codex1 V2 review orchestration. Use when $execute finishes a task, when a review bundle is open, or when the parent is routing repair/replan.
---

# $review-loop (Codex1 V2)

Parent-owned orchestration over reviewer outputs. The parent opens bundles,
submits reviewer outputs, and closes bundles; reviewers themselves are
bounded subagents that only produce findings — they never clear gates.

## When to use

- A task is in `proof_submitted` and `review_required[]` names it.
- A review bundle is `open` and has fewer than `min_outputs` for a
  requirement.
- The last close produced findings and the task routed to `needs_repair`.

## Binary resolver

Every skill starts by resolving the V2 `codex1` binary to `$CODEX1`.

```bash
CODEX1="$("${CODEX1_REPO_ROOT:-/Users/joel/codex1}/scripts/resolve-codex1-bin")" || {
  echo "V2 codex1 not found. Set CODEX1_REPO_ROOT=<codex1 checkout> or build with: cargo build -p codex1 --release" >&2
  exit 1
}
```

Use `"$CODEX1"` for every `codex1` invocation below.

## Steps

1. Activate the review parent loop:
   ```bash
   "$CODEX1" parent-loop activate --mission <id> --mode review --json
   ```

2. Open the bundle:
   ```bash
   "$CODEX1" review open --mission <id> --task <task_id> \
     --profiles code_bug_correctness,local_spec_intent --json
   ```
   Capture the returned `bundle_id`.

3. Dispatch reviewer subagents. Each reviewer produces a `ReviewerOutput`
   JSON (schema in `docs/codex1-v2-cli-contract.md`) bound to the
   bundle's `task_run_id`, `graph_revision`, `state_revision`, and
   `evidence_snapshot_hash`. Save each as a JSON file.

4. Submit each reviewer output:
   ```bash
   "$CODEX1" review submit --mission <id> --bundle <B> --input path/to/out.json --json
   ```
   Stale bindings → `STALE_OUTPUT`. Parent-role outputs → refused.

5. Check cleanliness:
   ```bash
   "$CODEX1" review status --mission <id> --bundle <B> --json
   ```

6. Close the bundle:
   ```bash
   "$CODEX1" review close --mission <id> --bundle <B> --json
   ```
   Clean → task → `review_clean`. Findings → task → `needs_repair`.

7. If `needs_repair`, hand back to `$execute` to re-run. If
   `"$CODEX1" replan check` flags a mandatory trigger, hand to `$plan`.

8. When no reviews are open and no tasks owe review, **task-level
   review is done — do NOT deactivate the parent loop here.** Hand off
   to `$close` so the mission-close review and `mission-close complete`
   can run under the same active loop. `$close` is responsible for
   `parent-loop deactivate` after terminal completion. Dropping Ralph
   pressure before terminal close recreates V1's "final clean frontier
   stalls before mission close" failure mode.

## Stop boundaries

- **No parent self-review.** `review submit` refuses outputs whose
  `reviewer_role` equals the bundle's `opener_role` (`parent`).
- **No reviewer gate-clearing.** Only `review close` can mark a bundle
  clean or failed — reviewers only produce findings.
- **P0/P1/P2 block clean.** P3 is recorded but does not block.
- **Six-consecutive-non-clean rule.** After six failed closures on the
  same task, `"$CODEX1" replan check` flags a mandatory replan trigger.
