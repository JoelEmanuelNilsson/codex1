# Round 5 — handoff-cross-check audit

## Summary

Audited `HEAD = b08d461` against every normative claim and anti-goal
across the seven handoff docs at
`docs/codex1-rebuild-handoff/{README,00-why-and-lessons,01-product-flow,02-cli-contract,03-planning-artifacts,04-roles-models-prompts,05-build-prompt}.md`.

**0 P0, 0 P1, 0 P2, 0 P3.**

Rule-5 reminder: round-1 REJECTed the `02-cli-contract.md:219` suggested-
verdict `complete` vs code `terminal_complete` drift (handoff-internal;
the handoff is frozen for this loop and `00-why-and-lessons.md:172` /
`state::readiness.rs:31` both say `terminal_complete`). Round-2 REJECTed
`ParsedPlan#[serde(deny_unknown_fields)]` as non-blocking P3. Neither is
re-surfaced here.

### Anti-goals

Every declared anti-goal (`README.md:82-96`, `00-why-and-lessons.md:77-90`,
`02-cli-contract.md:108-121`, `05-build-prompt.md:30-39`) is honored:

- **No `.ralph/` directory.** `find /Users/joel/codex1 -name ".ralph"
  -o -name ".ralph*"` returns zero hits. Mentions survive only in
  anti-goal prose (`.codex/skills/close/SKILL.md`, `crates/codex1/src/lib.rs`,
  every handoff doc, `README.md:3`).
- **No caller-identity predicates in Rust.** Grep for
  `is_parent|is_subagent|caller_type|reviewer_id|parent_id|subagent_type|caller_identity|session_owner|session_token|capability_token|authority_token`
  in `crates/codex1/src/**/*.rs` returns zero matches. Every gated
  mutation rests on artifact state (`plan.locked`, `close.terminal_at`,
  `replan.triggered`, review classification), not on who called.
- **No stored waves.** `ParsedPlan`
  (`crates/codex1/src/cli/plan/parsed.rs:10-22`) has no `waves` field.
  `MissionState` / `PlanState`
  (`crates/codex1/src/state/schema.rs:71-86,177-194`) have no `waves`
  field. `plan scaffold` (`cli/plan/scaffold.rs::render_skeleton:119-171`)
  and `init` (`cli/init.rs::write_plan_template:125-153`) emit no
  `waves:` key. Wave derivation happens at `cli/plan/waves.rs` and
  `cli/status/next_action.rs` from `depends_on` + state.
- **No reviewer writeback.** Only `codex1 review record`
  (`crates/codex1/src/cli/review/record.rs`) and
  `codex1 close record-review` (`crates/codex1/src/cli/close/record_review.rs`)
  transition review state; both are main-thread-invoked. Skill prompts
  at `.codex/skills/review-loop/SKILL.md:69` make this explicit:
  "Reviewer writeback is forbidden."
- **No per-task lane state beyond status/proof.** `TaskRecord`
  (`crates/codex1/src/state/schema.rs:99-111`) carries only
  `id/status/started_at/finished_at/proof_path/superseded_by` — no
  lane/lock/reviewer identity fields. `ReviewRecord:130-141` carries
  `reviewers: Vec<String>` as a display csv (see `02-cli-contract.md:417`
  `--reviewers code-reviewer,intent-reviewer`), not as an authority token;
  `boundary_revision` is late-output classification (pinned against the
  pre-mutate revision in `cli/review/classify.rs`), not identity.

Adjacent non-goals held:

- **No wrapper runtime.** Grep for `Command::new` / `tokio::spawn` /
  `daemon` / `spawn_process` in `crates/codex1/src` returns zero matches.
- **Ralph is status-only.** `scripts/ralph-stop-hook.sh:25-37` runs
  exactly `codex1 status --json` and dispatches on `.data.stop.allow`.
  Matches `01-product-flow.md:232-243`.

### Structural invariants

- **6 skills + SKILL.md + `agents/openai.yaml`.** All six required
  skills (`autopilot`, `clarify`, `close`, `execute`, `plan`,
  `review-loop`) exist at `.codex/skills/<name>/`. Every one has both
  `SKILL.md` and `agents/openai.yaml`. Matches `01-product-flow.md:7-17`.
- **Mission layout under `PLANS/<mission-id>/`.** `cli/init.rs:44-46`
  creates mission scaffold via `state::init_write` +
  `write_outcome_template` + `write_plan_template`. Populates
  `OUTCOME.md`, `PLAN.yaml`, `STATE.json`, `EVENTS.jsonl`, `specs/`,
  `reviews/` (`state/mod.rs:157-179`). `CLOSEOUT.md` written by
  `cli/close/complete.rs:81-82`. Matches `03-planning-artifacts.md:9-26`.
- **Atomic-write protocol.** `state/fs_atomic.rs:22-37` follows
  tempfile-in-dir → `sync_data` → `persist` (rename) → parent-dir
  `sync_all`. Matches `02-cli-contract.md:385-391`. Wrapped under an
  exclusive fs2 lock inside `state::mutate` (`state/mod.rs:91-152`)
  with the documented "append EVENTS.jsonl FIRST, then persist
  STATE.json" ordering preserved (rationale inline at
  `state/mod.rs:126-141`).
- **Mission resolution precedence.** `core/mission.rs:28-51` implements
  `--mission + --repo-root` → `--mission` alone (CWD) → `--repo-root`
  alone (`discover_single_mission`) → CWD walk-up
  (`walk_up_for_mission`). Matches the docstring precedence at
  `core/mission.rs:3-6` and the handoff expectation at
  `02-cli-contract.md:39-44` ("Supports `--mission`… `--repo-root`…
  avoid hidden active-mission resolution").
- **Verdict derivation ordering.** `state/readiness.rs:40-66`:
  `TerminalComplete` → `NeedsUser(!outcome.ratified)` →
  `NeedsUser(!plan.locked)` → `Blocked(replan.triggered)` →
  `Blocked(has_blocking_dirty)` → mission-close family
  (`ReadyForMissionCloseReview` / `MissionCloseReviewOpen` /
  `MissionCloseReviewPassed`) when `tasks_complete` → `ContinueRequired`.
  Consumed identically by `status` (`cli/status/project.rs:19`) and
  `close check` (`cli/close/check.rs:47-49`) through
  `ReadinessReport::from_state`, so the two endpoints cannot diverge —
  the handoff invariant at `02-cli-contract.md:208` ("status and close
  check must share readiness logic"). Suggested verdict vocabulary at
  `02-cli-contract.md:212-221` and `00-why-and-lessons.md:166-176`
  matches `Verdict::as_str` at `readiness.rs:22-34`.
- **EVENTS.jsonl append-only + monotonic seq.** `state/events.rs:41-47`
  opens with `OpenOptions::new().create(true).append(true).open(path)`.
  `seq = state.events_cursor`, bumped exactly once per successful
  mutation (`state/mod.rs:125`) and never rewound. No code path writes
  historical events.
- **Hard-planning evidence enforced.** `cli/plan/check.rs:273-309`
  rejects `effective: hard` plans unless `planning_process.evidence`
  is non-empty AND at least one entry has
  `kind ∈ {explorer, advisor, plan_review}` (`HARD_EVIDENCE_KINDS` in
  `cli/plan/parsed.rs:112`). Matches `02-cli-contract.md:349-350` and
  `03-planning-artifacts.md:280-287`.

All 14 handoff-suggested error codes (`02-cli-contract.md:520-534`) are
present at `core/error.rs:83-96` as `CliError` variants with stable
`code()` strings: `OUTCOME_INCOMPLETE`, `OUTCOME_NOT_RATIFIED`,
`PLAN_INVALID`, `DAG_CYCLE`, `DAG_MISSING_DEP`, `TASK_NOT_READY`,
`PROOF_MISSING`, `REVIEW_FINDINGS_BLOCK`, `REPLAN_REQUIRED`,
`CLOSE_NOT_READY`, `STATE_CORRUPT`, `REVISION_CONFLICT`,
`STALE_REVIEW_RECORD`, `TERMINAL_ALREADY_COMPLETE`. Reserved variants
(`CONFIG_MISSING`, `MISSION_NOT_FOUND`, `PARSE_ERROR`, `NOT_IMPLEMENTED`)
round-trip through `to_envelope()` under round-1 unit coverage at
`core/error.rs::tests`.

Skill model matrix matches handoff (`04-roles-models-prompts.md:21-22,163-167`
and `05-build-prompt.md:97-100`):
`.codex/skills/review-loop/SKILL.md:57-63` lists
`code_bug_correctness → gpt-5.3-codex high`,
`local_spec_intent → gpt-5.4 high`,
`integration_intent → gpt-5.4 high`,
`plan_quality → gpt-5.4 high|xhigh`,
`mission_close → gpt-5.4 high (2 lanes)`. No drift.

## Round-5 spot checks

### Spot check 1 — `loop activate` is the sole writer of `loop.active = true`

Handoff claim: `02-cli-contract.md:96-98` — "`loop activate` is the
canonical entry point other subsystems use to set `state.loop.active =
true` without inventing their own loop-state mutation."

Audit path: grep for every assignment site that could flip
`state.loop_.active` to `true`.

- `grep -n "s\.loop_ = \| state\.loop_ ="` in `crates/codex1/src`
  returns five production sites:
  - `cli/loop_/mod.rs:105` — `s.loop_ = target;` inside
    `run_transition`; the `target` value is classified by the
    per-subcommand closure. The `Activate` subcommand routes through
    `cli/loop_/activate.rs:11-25`, which is the only production site
    that constructs a `LoopState` literal with `active: true` (see
    `LoopState { active: true, paused: false, mode }` at `:14-18`).
    Pause / resume / deactivate paths at
    `cli/loop_/pause.rs`, `cli/loop_/resume.rs`,
    `cli/loop_/deactivate.rs` construct targets with `active: …`
    matching their own semantics (pause keeps active=true but flips
    paused; deactivate sets active=false; resume starts from
    `Transition::Reject` when already active-and-unpaused, and keeps
    `active: true` when clearing paused — no net activation event).
  - `cli/close/complete.rs:72` — `state.loop_ = LoopState { active:
    false, paused: false, mode: LoopMode::None };`. This is a
    deactivation on terminal close, not an activation, so it does not
    contradict the claim.
  - `state/readiness.rs:253,260,267,274` — all are inside
    `#[cfg(test)] mod tests`, so they are unit-test seeds only.

All three additional independent greps (`plan\.locked`,
`close\.review_state =`, `replan\.triggered =`) converge on the same
shape: every authoritative write is one subsystem-per-write. No
"side setter" exists for `loop.active = true` outside
`cli/loop_/activate.rs`. **Honored.**

### Spot check 2 — `close record-review` is the only writer of `MissionCloseReviewState::Open → Passed`

Handoff claim: `02-cli-contract.md:100-104` — "`close record-review`
records the main-thread outcome of the mission-close review — it is
the only write path that transitions `MissionCloseReviewState::Open →
Passed`."

Audit path: grep `MissionCloseReviewState::Passed` across
`crates/codex1/src` and enumerate each occurrence as "assignment" vs
"match arm / test seed / read":

```
crates/codex1/src/state/readiness.rs:61    => Verdict::MissionCloseReviewPassed  (read, enum→verdict map)
crates/codex1/src/state/readiness.rs:229   review_state: … (test seed)
crates/codex1/src/state/readiness.rs:239   s.close.review_state = … (test seed, cfg(test))
crates/codex1/src/cli/close/closeout.rs:83  => "clean"  (match arm, rendering)
crates/codex1/src/cli/close/closeout.rs:101 =>          (match arm, rendering)
crates/codex1/src/cli/close/check.rs:146   => {}        (match arm, blocker enumeration)
crates/codex1/src/cli/close/record_review.rs:103  (argument to emit_success on dry-run)
crates/codex1/src/cli/close/record_review.rs:120  state.close.review_state = MissionCloseReviewState::Passed;  (ASSIGNMENT)
crates/codex1/src/cli/close/record_review.rs:132  (argument to emit_success)
```

The only production-code assignment (outside the `#[cfg(test)]`
readiness-unit-test module at `readiness.rs:229,239`) is
`cli/close/record_review.rs:120`, inside `record_clean`, which runs
under `state::mutate` with event `"close.review.clean"`. The
corresponding `Open` assignment (`record_dirty:205`) is co-located.
No other Rust code path can transition a mission-close review to
`Passed`. **Honored.**

### Spot check 3 — 14 error codes present verbatim at HEAD

Handoff claim: `02-cli-contract.md:520-534` lists 14 suggested codes.

Code at `core/error.rs:78-103` — `CliError::code()` match returns each
stable string:

| Handoff code | `CliError::code()` site |
|---|---|
| `OUTCOME_INCOMPLETE` | `error.rs:83` |
| `OUTCOME_NOT_RATIFIED` | `error.rs:84` |
| `PLAN_INVALID` | `error.rs:85` |
| `DAG_CYCLE` | `error.rs:86` |
| `DAG_MISSING_DEP` | `error.rs:87` |
| `TASK_NOT_READY` | `error.rs:88` |
| `PROOF_MISSING` | `error.rs:89` |
| `REVIEW_FINDINGS_BLOCK` | `error.rs:90` |
| `REPLAN_REQUIRED` | `error.rs:91` |
| `CLOSE_NOT_READY` | `error.rs:92` |
| `STATE_CORRUPT` | `error.rs:93` |
| `REVISION_CONFLICT` | `error.rs:94` |
| `STALE_REVIEW_RECORD` | `error.rs:95` |
| `TERMINAL_ALREADY_COMPLETE` | `error.rs:96` |

All 14 handoff codes are present as distinct variants with exact
string match. Four non-handoff-listed reserved codes
(`CONFIG_MISSING`, `MISSION_NOT_FOUND`, `PARSE_ERROR`,
`NOT_IMPLEMENTED`) coexist without displacing any handoff code —
round-1 test-adequacy P1-1/P1-2 kept them after debate. **Honored.**

## Round-1/2/3/4 fix verifications (no regressions)

All prior fixes remain intact at HEAD:

- `state::require_plan_locked` pre-mutate calls
  (`cli/task/start.rs:22`, `cli/task/finish.rs:21`,
  `cli/review/start.rs:41`, `cli/review/record.rs:77`) and in-closure
  re-checks (`cli/task/start.rs:111`, `cli/task/finish.rs:112`,
  `cli/review/start.rs:95`, `cli/review/record.rs:185-187`) — present.
- EVENTS-before-STATE ordering in `state::mutate`
  (`state/mod.rs:142-145`) — present with rationale.
- Parent-dir `sync_all` after `persist` (`state/fs_atomic.rs:33-35`) —
  present.
- `outcome ratify` state-first-then-OUTCOME.md atomicity
  (`cli/outcome/ratify.rs:60-75`) — preserved.
- `--expect-revision` enforcement on every short-circuit:
  `task/start.rs`, `task/finish.rs`, `plan/check.rs:78`,
  `plan/choose_level.rs:53`, `review/start.rs` (dry-run via
  `check_expected_revision` — round-2 correctness P2-1),
  `review/record.rs:101`, `close/complete.rs:53`,
  `close/record_review.rs:95,154`, `outcome/ratify.rs:34`,
  `loop_/mod.rs:75,79,92`.
- Six-consecutive-dirty replan threshold `= 6`
  (`cli/review/record.rs:29 DIRTY_STREAK_THRESHOLD`,
  `cli/close/record_review.rs:26 DIRTY_REPLAN_THRESHOLD`).
- `cli/plan/check.rs:133-134` clears `state.replan.triggered` +
  `triggered_reason` at relock (round-2 e2e P0-1). Rationale inline at
  `check.rs:125-132`.
- `cli/task/next.rs:26-53` short-circuits on `!plan.locked` and
  `replan.triggered`, mirroring `cli/status/project.rs::derive_next_action`.
- `.codex/skills/plan/SKILL.md:190` and
  `.codex/skills/plan/references/dag-quality.md:46` replan-record
  snippets carry `--reason`.
- `.codex/skills/plan/SKILL.md:20` says `outcome_ratified`;
  `.codex/skills/execute/SKILL.md:16` says `plan_locked` — matches
  the flat status projection at `cli/status/project.rs:72-73`.
- `.codex/skills/review-loop/SKILL.md:39-43` and
  `.codex/skills/review-loop/references/reviewer-profiles.md` preserve
  the expanded mission-close dispatch + aligned reviewer matrix.
- `cli/outcome/ratify.rs` preserves the closing-fence newline
  unconditionally (round-3 e2e P1-1); idempotent + hand-written
  OUTCOME.md cases covered by `tests/outcome.rs`.
- Direct `STATE_CORRUPT` integration test at
  `tests/foundation.rs::state_corrupt_envelope_on_invalid_state_json`
  covers the `serde_json::from_str` branch at `state/mod.rs:84,111`
  (round-3 test-adequacy P2-1).
- `full_mission_close_after_replan_reaches_terminal` asserts
  CLOSEOUT.md presence + body shape (round-3 test-adequacy P2-2).
- `cli/review/packet.rs` parses OUTCOME.md frontmatter via
  `serde_yaml::from_str` so YAML block-scalar indicators (`|`) no
  longer leak into `mission_summary` (round-4 cli-contract/e2e P2-1).
- `cli/review/record.rs` → `CliError::ReviewFindingsBlock` envelope
  integration-tested at
  `tests/review.rs::review_record_findings_then_retry_returns_review_findings_block_envelope`
  (round-4 test-adequacy P2-1).

## P0

None.

## P1

None.

## P2

None.

## P3

None.
