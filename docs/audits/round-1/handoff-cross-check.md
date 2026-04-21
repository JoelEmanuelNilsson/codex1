# Round 1 — handoff-cross-check audit

## Summary

Audited `HEAD = 73c1285` (round-0 baseline, forward of the iter4-clean `5a16894`) against every normative claim and anti-goal in the seven handoff docs under `docs/codex1-rebuild-handoff/`.

**0 P0, 0 P1, 1 P2, 3 P3.**

Every declared anti-goal is honored in code, skills, and scripts:

- `.ralph/` appears only as anti-goal prose or explicit prohibition (CC-1 of prior audit still holds; all 11 matches are docs/skills/lib.rs doc-comments).
- Caller-identity patterns (`is_parent|is_subagent|caller_type|reviewer_id|parent_id|subagent_type|caller_identity|session_owner|session_id|session_token|capability_token|authority_token`) — zero matches in `crates/codex1/src/**/*.rs`.
- `plan scaffold` (`crates/codex1/src/cli/plan/scaffold.rs::render_skeleton` L119-171) emits no `waves:` key; no plan-file template in `init.rs` or `scaffold.rs` carries one.
- `MissionState` (`crates/codex1/src/state/schema.rs::MissionState` L177-194) has no `waves` field; `TaskRecord` (L99-111) has only `status`/proof pointers — no per-task lane state or lock.
- Reviewer writeback is positively forbidden: `.codex/skills/review-loop/SKILL.md:11,63,79` + `references/reviewer-profiles.md:8-15`.
- No wrapper runtime: zero `Command::new` / `tokio::spawn` / `daemon` / `spawn_process` hits in `crates/codex1/src`.
- Ralph hook (`scripts/ralph-stop-hook.sh:25`) runs exactly one command: `codex1 status --json`.

Structural invariants:

- Six skills exist at `.codex/skills/{autopilot,clarify,close,execute,plan,review-loop}/`, each with `SKILL.md` + `agents/openai.yaml` (no `agents/openai.yaml` is missing).
- Mission files under `PLANS/<mission-id>/` resolved by `crates/codex1/src/core/paths.rs` — `OUTCOME.md`, `PLAN.yaml`, `STATE.json`, `STATE.json.lock`, `EVENTS.jsonl`, `CLOSEOUT.md`, `specs/`, `reviews/`. `codex1 init` (`crates/codex1/src/cli/init.rs:44-46`) creates all files at init time except `CLOSEOUT.md` (written by `close complete`, per handoff 03-planning-artifacts.md:43).
- Atomic-write protocol (`crates/codex1/src/state/fs_atomic.rs:18-28` tempfile-plus-rename) and `mutate` (`crates/codex1/src/state/mod.rs:52-97` — fs2 exclusive lock → read → check revision → bump → atomic write → append event → unlock) match 02-cli-contract.md:385-391 and 03-planning-artifacts.md.
- Mission resolution precedence (`crates/codex1/src/core/mission.rs:28-51,84-135`) follows handoff: `--mission + --repo-root` → `--mission` alone → CWD ancestor walk → `discover_single_mission` (error on 0 or >1).
- `derive_verdict` (`crates/codex1/src/state/readiness.rs:40-66`) ordering — `TerminalComplete` → `NeedsUser` (outcome) → `NeedsUser` (plan) → `Blocked` (replan) → `Blocked` (dirty) → `ReadyForMissionCloseReview` / `Open` / `Passed` → `ContinueRequired` — is used identically by `status` (`crates/codex1/src/cli/status/project.rs:19`) and `close check` (`crates/codex1/src/cli/close/check.rs:47`), so 02-cli-contract.md:208 ("status and close check must share readiness logic") holds.
- `EVENTS.jsonl` is append-only (`crates/codex1/src/state/events.rs:41-47` uses `OpenOptions::append(true)`) with monotonic `seq` = `events_cursor` bumped once per `state::mutate`.
- Hard-planning evidence (`crates/codex1/src/cli/plan/check.rs:250-286`) rejects `effective: hard` plans without an evidence entry of kind in `{explorer, advisor, plan_review}`.
- Six-consecutive-dirty replan trigger: `crates/codex1/src/cli/review/record.rs:28 DIRTY_STREAK_THRESHOLD = 6`; `apply_dirty` increments, `apply_clean` resets to 0 (L302). Mirrored in `crates/codex1/src/cli/close/record_review.rs:26` for mission-close reviews.
- Mission-close review is mandatory before `close complete`: `crates/codex1/src/cli/close/complete.rs:41-46` refuses unless `ReadinessReport::ready`, which requires `Verdict::MissionCloseReviewPassed`, which requires `MissionCloseReviewState::Passed` — only reachable via `codex1 close record-review --clean`.

## P0

None.

## P1

None.

## P2

### F1 — `choose-level` emits `escalation_reason` when requested == effective

Citation: `docs/codex1-rebuild-handoff/02-cli-contract.md:325` vs `crates/codex1/src/cli/plan/choose_level.rs:34-37, 153-157`.

Evidence: 02-cli-contract.md:325 is a normative Implementation rule: "Include `escalation_reason` only when effective level is higher than requested level." The code unconditionally attaches `escalation_reason` whenever `--escalate` is supplied, even though supplying `--escalate` also pins `effective = Hard` regardless of the requested level:

```rust
// choose_level.rs:34-37
let (effective, escalation_reason) = match escalate {
    Some(reason) => (PlanLevel::Hard, Some(reason)),
    None => (requested.clone(), None),
};
```

```rust
// choose_level.rs:153-157  (inside build_payload)
if let Some(reason) = escalation_reason {
    data.as_object_mut()
        .expect("object")
        .insert("escalation_reason".to_string(), json!(reason));
}
```

Reproducer: `codex1 plan choose-level --level hard --escalate "already hard" --json` produces `{"requested_level":"hard","effective_level":"hard","escalation_reason":"already hard",...}` — the rule requires `escalation_reason` to be omitted when `requested == effective`. The JSON output contract therefore disagrees with the handoff's implementation rule. Also surfaces in `PLAN.yaml` only indirectly (scaffold writes a `[codex1-fill:…]` block for `escalation_reason`), but the CLI envelope is the primary consumer.

Suggested fix: guard the payload insertion and the event payload on `requested != effective` (or on `effective as rank > requested as rank`). Also clear `escalation_reason` when the `--escalate` flag is passed at the max level so the stored state does not silently carry a phantom reason.

## P3

### F2 — Reviewer profile model recommendations drift from handoff matrix

Citation: `docs/codex1-rebuild-handoff/04-roles-models-prompts.md:163-167` and `docs/codex1-rebuild-handoff/05-build-prompt.md:97-100` vs `.codex/skills/review-loop/SKILL.md:53-54` + `.codex/skills/review-loop/references/reviewer-profiles.md:68, 88`.

Evidence: handoff model matrix says `code_bug_correctness → gpt-5.3-codex` and `local_spec_intent → gpt-5.4`. Review-loop skill lists `claude-opus-4-7` for `code_bug_correctness` and `claude-opus-4-7` or `gpt-5.4` for `local_spec_intent`. Same text repeats in `reviewer-profiles.md:68` ("Model: claude-opus-4-7, reasoning high") and `:88` ("Model: claude-opus-4-7 or gpt-5.4").

The handoff permits model substitutions ("Use this matrix unless a future model change makes a replacement obviously better" — 04-roles-models-prompts.md:10-12), so this is soft drift rather than a contract violation, but the skill is the consumer-facing prompt and should either pin the matrix or state why it replaced it.

Suggested fix: either (a) align the skill table back to `gpt-5.3-codex`/`gpt-5.4` as the primary recommendation, or (b) update the handoff matrix to name `claude-opus-4-7` as an accepted peer for `code_bug_correctness`. Do not leave the two artifacts silently disagreeing.

### F3 — Suggested verdict list in CLI-contract doc names `complete` but the code (and other handoff docs) use `terminal_complete`

Citation: `docs/codex1-rebuild-handoff/02-cli-contract.md:212-221` vs `docs/codex1-rebuild-handoff/00-why-and-lessons.md:173`, `crates/codex1/src/state/readiness.rs:31`.

Evidence: 02-cli-contract.md suggested list includes `complete`. 00-why-and-lessons.md § Mission-Close Vocabulary (L166-176) names the terminal state `terminal_complete`. Code (`readiness.rs:31`) emits `"terminal_complete"`. The code matches 00-why-and-lessons.md; the 02 list is the outlier. This is handoff-doc-internal drift; no runtime divergence.

Suggested fix: update 02-cli-contract.md:216 from `complete` to `terminal_complete` so the verdict list agrees with 00-why-and-lessons.md and the code. (Or, alternatively, add `complete` as an alias — but the current code never emits it, so dropping it from the doc is cleaner.)

### F4 — Handoff example shows `escalation_required: true` field that the CLI never emits

Citation: `docs/codex1-rebuild-handoff/02-cli-contract.md:306-315` vs `crates/codex1/src/cli/plan/choose_level.rs:139-159`.

Evidence: the escalation-example JSON in 02-cli-contract.md includes `"escalation_required": true`. The CLI payload builder emits `requested_level`, `effective_level`, optional `escalation_reason`, and `next_action` — no `escalation_required` field. The handoff's "Implementation rules" (L319-326) do not list `escalation_required` as required output, so the example is aspirational rather than contractual; consumers relying on the example JSON shape would be surprised.

Suggested fix: either add an `"escalation_required": bool` line to `build_payload` when `effective > requested` (cheap, matches the example), or strike the field from the 02-cli-contract.md example so docs and code agree.
