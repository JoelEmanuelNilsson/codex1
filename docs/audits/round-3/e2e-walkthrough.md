# Round 3 — e2e-walkthrough audit

## Summary

Built `~/.local/bin/codex1` via `make -C /Users/joel/codex1 install-local` and drove full missions from `/tmp/codex1-review-e2e-r3/`.

- **Happy path verified.** `init → outcome ratify → plan choose-level → plan scaffold → (re-seed PLAN.yaml) → plan check → task start/finish (×N) → review start/record → close record-review --clean → close complete` reached `verdict:"terminal_complete"`, `phase:"terminal"`, `stop.allow=true/reason:"terminal"`, and CLOSEOUT.md was written at `/tmp/codex1-review-e2e-r3/PLANS/simple/CLOSEOUT.md`. Revision monotonicity held across every mutation (demo: rev 0→4 through plan-lock; simple: rev 0→9 to terminal).
- **Round-3 regression checks — all pass.**
  1. After `replan record --reason user_request --supersedes T2` → `plan check`, STATE.json shows `state.replan.triggered == false` and the `triggered_reason` field is absent (serde `skip_serializing_if = Option::is_none` → equivalent to null). Verified on `/tmp/codex1-review-e2e-r3/PLANS/replanner/STATE.json` at rev 9.
  2. Full replan cycle reaches `terminal_complete`. The pre-existing integration test `tests/e2e_replan_trigger.rs::full_mission_close_after_replan_reaches_terminal` passes under `cargo test -p codex1 --test e2e_replan_trigger --release`.
  3. `task next` after `replan record` returns `{"kind":"plan","hint":"Draft and lock PLAN.yaml."}` (manual reproduction on `replanner` at rev 8). The `plan.locked && replan.triggered` branch returns `{"kind":"replan","reason":…}` as verified by `tests/status.rs::task_next_replan_triggered_emits_replan_kind` (passes under `cargo test --release`). Short-circuit source: `crates/codex1/src/cli/task/next.rs:26-53`.
  4. `review start T4 --dry-run --expect-revision 999` returns code `REVISION_CONFLICT`, retryable, with context `{expected:999, actual:11}` (verified live on `replanner`).
  5. `tests/foundation.rs::concurrent_replan_and_task_start_preserves_plan_locked_invariant` passes — round-2 TOCTOU fix holds.
- **Ralph stop hook verified.** `scripts/ralph-stop-hook.sh` exits 2 on an active loop with `verdict:"continue_required"` (reproduced on `/tmp/single-mission-test` after `loop activate` + ready task) with stderr "blocking Stop - reason=active_loop". Exits 0 on a terminal mission (`/tmp/terminal-hook-test` post-`close complete`). `cargo test -p codex1 --test ralph_hook --release` passes all 6 tests.
- **Error envelopes exercised end-to-end.** Observed codes `MISSION_NOT_FOUND`, `OUTCOME_INCOMPLETE`, `OUTCOME_NOT_RATIFIED` (implicit via `plan check` before ratify), `PLAN_INVALID` (missing spec file, unknown supersedes target, unknown reason), `DAG_CYCLE`, `DAG_MISSING_DEP`, `TASK_NOT_READY`, `PROOF_MISSING` (implicit), `REVIEW_FINDINGS_BLOCK`, `STALE_REVIEW_RECORD`, `REVISION_CONFLICT`, `TERMINAL_ALREADY_COMPLETE`, `CLOSE_NOT_READY`. All envelopes carry the expected `{ok:false, code, message, hint?, retryable, context?}` shape.
- **Tooling clean.** `cargo fmt --check`, `cargo clippy --release --all-targets`, and `cargo test -p codex1 --release` (all 18 suites) pass on baseline `95451e6`.

One new P1 finding: `outcome ratify` corrupts OUTCOME.md (rewrite drops / merges the newline after the closing `---` fence). Not previously flagged in rounds 1 or 2.

## P0

None.

## P1

### P1-1 · `outcome ratify` corrupts OUTCOME.md on re-ratify or when body has no leading blank line

**Type:** (b) reproducible bug with exact command.

**Code site.** `crates/codex1/src/cli/outcome/ratify.rs:105-143` — `rewrite_status_to_ratified` rebuilds the file as:
```rust
let mut out = String::with_capacity(frontmatter.len() + body.len() + 8);
out.push_str("---\n");
out.push_str(&new_front);
out.push_str("---");
// Preserve the exact body, including any leading newline that was
// part of the file (split_frontmatter leaves `\n# Body…` in `body`).
out.push_str(body);
```

Line 138 emits `"---"` (no trailing newline) on the assumption that `body` begins with `\n`. That assumption is false whenever the original file has the shape `…\n---\n# Heading…` (closing fence directly followed by a body line, no blank line between). In that case `split_frontmatter` at `crates/codex1/src/cli/outcome/validate.rs:166-168` returns `body = rest.get(body_start..)` where `body_start = line_end` = just past the closing `---\n`. So `body` starts with `# Heading` (no leading `\n`), and the rewrite concatenates `"---"` + `"# Heading…"` into `"---# Heading…"` on a single line — the closing fence disappears as a standalone line.

**Reproducer A (single successful ratify on reasonable user input).**

```bash
rm -rf /tmp/no-blank && mkdir /tmp/no-blank
cd /tmp/no-blank && ~/.local/bin/codex1 init --mission m > /dev/null
# Hand-written OUTCOME.md with no blank line between closing fence and body
cat > /tmp/no-blank/PLANS/m/OUTCOME.md <<'EOF'
---
mission_id: m
status: draft
title: T
original_user_goal: |
  x
interpreted_destination: |
  x
must_be_true:
  - x
success_criteria:
  - x
non_goals:
  - x
constraints:
  - x
quality_bar:
  - x
proof_expectations:
  - x
review_expectations:
  - x
known_risks:
  - x
resolved_questions: []
---
# OUTCOME
body.
EOF
~/.local/bin/codex1 --repo-root /tmp/no-blank --mission m outcome ratify   # ok:true
~/.local/bin/codex1 --repo-root /tmp/no-blank --mission m outcome check    # ok:false OUTCOME_INCOMPLETE
# File now contains literally "---# OUTCOME" on one line:
grep -c '^---# OUTCOME' /tmp/no-blank/PLANS/m/OUTCOME.md                    # 1
```

One successful `outcome ratify` on a user-written OUTCOME.md that lacks a blank line between the closing fence and the body collapses the fence into the heading, making the file unparseable. A subsequent `outcome check`, `outcome ratify`, or anything else that reads the frontmatter returns `OUTCOME_INCOMPLETE`.

**Reproducer B (double ratify on the init template).** Even with the scaffolded template (`crates/codex1/src/cli/init.rs:72-119`), which does insert a blank line, two successive ratifies break the file because the first ratify silently strips the blank line:

```bash
rm -rf /tmp/init-tmpl && mkdir /tmp/init-tmpl
cd /tmp/init-tmpl && ~/.local/bin/codex1 init --mission m > /dev/null
sed -i '' 's/\[codex1-fill:[^]]*\]/filled/g' /tmp/init-tmpl/PLANS/m/OUTCOME.md
~/.local/bin/codex1 --repo-root /tmp/init-tmpl --mission m outcome ratify   # ok:true, drops the blank line after `---`
~/.local/bin/codex1 --repo-root /tmp/init-tmpl --mission m outcome ratify   # ok:true, concatenates `---` with `# OUTCOME`
~/.local/bin/codex1 --repo-root /tmp/init-tmpl --mission m outcome check    # ok:false OUTCOME_INCOMPLETE
```

**Contract impact.** `docs/codex1-rebuild-handoff/02-cli-contract.md:49` says mutating commands "Must be idempotent where possible". `outcome ratify` violates this on two axes:

1. State-level: each call bumps `revision` and re-stamps `outcome.ratified_at`, and appends a fresh `outcome.ratified` event (two EVENTS.jsonl lines for the same logical transition).
2. File-level: each call strips one newline after the closing fence; one or two calls — depending on user input shape — turn OUTCOME.md into a file whose closing fence is no longer a standalone line, which `split_frontmatter` cannot detect.

The `.codex/skills/clarify/SKILL.md:56` repair flow instructs callers to "Repair, re-run `outcome check`, then ratify again" on `OUTCOME_INCOMPLETE`. That flow is safe only because the previous ratify attempt was unsuccessful (no file mutation on failure); but any caller that re-ratifies a successfully-ratified OUTCOME hits the bug. No guard at `ratify.rs:38-74` short-circuits when `state.outcome.ratified` is already `true`.

**Severity rationale (P1).** The second ratify does not brick the mission — STATE stays ratified, `status` still reports `outcome_ratified:true`, and subsequent `plan check / task / review / close` do not re-read OUTCOME.md. But `outcome check` is broken, user-authored OUTCOME.md content is silently lost, and `outcome ratify` is no longer idempotent in violation of the handoff contract. The no-blank-line variant (reproducer A) corrupts the file on a single successful call with reasonable hand-written input.

**Suggested fix.** In `rewrite_status_to_ratified`, unconditionally emit a newline after the closing fence and trim any leading newline the original body happened to carry:

```rust
out.push_str("---\n");
out.push_str(body.trim_start_matches('\n'));
```

Plus an idempotency check at `ratify.rs:38`: if `state.outcome.ratified && state.outcome.ratified_at.is_some()` and the current OUTCOME.md already carries `status: ratified`, short-circuit to a `JsonOk` that reuses the existing `ratified_at` without mutating state or the file.

## P2

None.

## P3

None.
