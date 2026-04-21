# Round 3 — skills-audit

## Summary

All six skills (`clarify`, `plan`, `execute`, `review-loop`, `close`, `autopilot`) pass `quick_validate.py`, hold frontmatter to `name`+`description` only, keep `SKILL.md` well under 500 lines, carry `agents/openai.yaml` with a `default_prompt` that references the `$<skill>` form, have no alongside `README.md`, and reference no `codex1 internal <verb>` commands. Every `codex1 <verb>` I sampled resolves to a real variant in `crates/codex1/src/cli/mod.rs` (Init, Status, Outcome{Check,Ratify}, Plan{ChooseLevel,Scaffold,Check,Graph,Waves}, Task{Next,Start,Finish,Packet,Status}, Review{Start,Packet,Record,Status}, Replan{Check,Record}, Loop{Activate,Pause,Resume,Deactivate}, Close{Check,Complete,RecordReview}).

Round-3 regression checks all pass:

- `plan/references/dag-quality.md:46` includes `--reason <code>` on `codex1 replan record` (confirmed).
- `plan/SKILL.md:20` reads `outcome_ratified: true` (flat) and `execute/SKILL.md:16` reads `plan_locked: true` (flat) — round-2 fix held.
- `execute/SKILL.md` step 1 dispatches on `data.next_action.kind` from `codex1 --json status` (round-1 fix held).
- `plan/SKILL.md:190` replan snippet includes `--reason <code>` (round-1 fix held).
- `review-loop/SKILL.md` and `references/reviewer-profiles.md` list `gpt-5.3-codex` for `code_bug_correctness` and `gpt-5.4` for `local_spec_intent` / `integration_intent` / `plan_quality` / `mission_close` (round-1 fix held).
- `autopilot/SKILL.md:63` and `references/autopilot-state-machine.md:24` both include the `fix_state` dispatch row (round-1 fix held).

One new P1 finding, no P0/P2/P3.

## P0

None.

## P1

### P1-1 · `review-loop` mission-close workflow stalls on `mission_close_review_open`

Finding shape: (a) handoff+skill divergence AND (b) reproducible bug.

`.codex/skills/review-loop/SKILL.md:39` (step 1 of the mission-close workflow):

> 1. **Confirm readiness.** Run `codex1 --json close check`. Require `data.verdict == ready_for_mission_close_review`. If not, stop.

After a dirty mission-close record, `cli/close/record_review.rs:205` sets `state.close.review_state = MissionCloseReviewState::Open`. `state/readiness.rs:60` then projects that to `Verdict::MissionCloseReviewOpen` (`mission_close_review_open`). `cli/status/project.rs:173-178` dispatches that verdict to `next_action.kind = "mission_close_review"` with hint "Mission-close review is open." `.codex/skills/autopilot/references/autopilot-state-machine.md:21` (row `mission_close_review_open → mission_close_review → $review-loop (mission-close mode)`) then routes control back into `$review-loop`.

The `$review-loop` skill refuses to proceed because `data.verdict` is now `mission_close_review_open`, not `ready_for_mission_close_review`. No repair tasks are generated (`dirty_repair_targets` at `cli/status/next_action.rs:130-159` scans `state.reviews`, which only contains *planned* review records — the mission-close record does not populate it), so nothing repopulates the path forward. The mission is stuck in `$autopilot` → `$review-loop` → "stop" until the six-dirty replan trigger eventually fires through `close record-review` from outside the skill flow, which cannot happen because the skill refuses to run reviewers.

Handoff expectation: `docs/codex1-rebuild-handoff/01-product-flow.md:188-194` describes mission-close mode as `review -> repair/replan -> review -> repair/replan -> clean`, i.e. successive review rounds are part of the flow. `docs/codex1-rebuild-handoff/02-cli-contract.md:210-221` lists both `ready_for_mission_close_review` AND `mission_close_review_open` as valid verdicts in the same set, implying the skill must accept either as an entry point to the mission-close review workflow.

Fix shape: step 1 should accept both `ready_for_mission_close_review` (first round) and `mission_close_review_open` (post-dirty round). The stop clause should fire only when the verdict indicates the mission is not at mission-close review time at all (e.g. `continue_required`, `needs_user`, `blocked` without `repair` / `replan`).

## P2

None.

## P3

None.
