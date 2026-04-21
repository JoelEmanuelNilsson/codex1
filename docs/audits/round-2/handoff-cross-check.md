# Round 2 — handoff-cross-check audit

## Summary

Audited `HEAD = 05fcae3` (round-1 fixes committed) against every normative
claim and anti-goal across the seven handoff docs at
`docs/codex1-rebuild-handoff/` and
`docs/codex1-rebuild-handoff/{00-why-and-lessons,01-product-flow,02-cli-contract,03-planning-artifacts,04-roles-models-prompts,05-build-prompt,README}.md`.

**0 P0, 0 P1, 0 P2, 1 P3.**

Every declared anti-goal is honored in code, skills, and scripts:

- `find /Users/joel/codex1 -name .ralph` returns zero hits. All `.ralph`
  mentions across the tree live in anti-goal prose or explicit prohibitions
  (`.codex/skills/close/SKILL.md:112`, `crates/codex1/src/lib.rs:9`,
  every `docs/codex1-rebuild-handoff/*.md`, `README.md:3`).
- `rg -n "is_parent|caller|identity|subagent_type|parent_id|reviewer_id|session_token|capability_token|authority_token" crates/codex1/src` returns only descriptive comments (`state/readiness.rs:76`, `state/mod.rs:12`, `state/events.rs:40`, `cli/loop_/mod.rs:72,131`, `cli/loop_/resume.rs:11`, `cli/review/packet.rs:89`, `cli/review/mod.rs:3`, `cli/outcome/validate.rs:24`, `cli/status/project.rs:25`, `cli/replan/record.rs:158`, `cli/plan/waves.rs:134`, `cli/plan/check.rs:156`, `cli/plan/choose_level.rs:5,6,37,111`, `cli/plan/dag.rs:4`, `lib.rs:8`, `cli/close/check.rs:73`). No Rust predicate anywhere branches on caller identity.
- `PLAN.yaml` parser `ParsedPlan` (`crates/codex1/src/cli/plan/parsed.rs:10-22`) has no `waves` field. `plan scaffold` template (`crates/codex1/src/cli/plan/scaffold.rs::render_skeleton:119-171`) emits no `waves:` key. `init.rs::write_plan_template:125-153` emits no `waves:` key either.
- `MissionState` (`crates/codex1/src/state/schema.rs:178-194`) has no `waves` field. `TaskRecord` (`schema.rs:100-111`) carries only `status`, lifecycle timestamps (`started_at`/`finished_at`), `proof_path`, `superseded_by` — no lane state, no lock token, no reviewer/identity field.
- Reviewer writeback is positively forbidden: `.codex/skills/review-loop/SKILL.md:11,63,79-84`, `.codex/skills/review-loop/references/reviewer-profiles.md:7-15`, `crates/codex1/src/cli/review/mod.rs:3`. The only write paths that transition review state are `codex1 review record` and `codex1 close record-review` — both main-thread-invoked.
- No wrapper runtime: zero `Command::new`, `tokio::spawn`, `daemon`, or `spawn_process` hits in `crates/codex1/src`.
- Ralph hook (`scripts/ralph-stop-hook.sh:25`) runs exactly one command: `codex1 status --json` — matches `01-product-flow.md:229-244`.

Structural invariants:

- Six skills exist at `.codex/skills/{autopilot,clarify,close,execute,plan,review-loop}/` with `SKILL.md` + `agents/openai.yaml`. Verified: all six `agents/openai.yaml` present.
- Mission files under `PLANS/<mission-id>/` are created by `codex1 init` (`crates/codex1/src/cli/init.rs:44-46`) and resolved by `crates/codex1/src/core/paths.rs`: `OUTCOME.md`, `PLAN.yaml`, `STATE.json`, `STATE.json.lock`, `EVENTS.jsonl`, `specs/`, `reviews/`. `CLOSEOUT.md` is written on terminal close (`crates/codex1/src/cli/close/complete.rs:81-82`).
- Atomic-write protocol matches handoff `02-cli-contract.md:385-391`: `crates/codex1/src/state/fs_atomic.rs:22-37` (tempfile-in-dir → `sync_data` → `persist` → parent-dir `sync_all`) plus `crates/codex1/src/state/mod.rs::mutate:91-152` (exclusive fs2 lock → read → check revision → run closure → bump → **append event first** → atomic write state → unlock). Ordering invariant commented at `state/mod.rs:126-141`.
- Mission resolution precedence (`crates/codex1/src/core/mission.rs:28-51,84-135`) follows handoff ordering: `--mission + --repo-root` → `--mission` alone → CWD-ancestor `PLANS/` walk-up → `discover_single_mission` (error on 0 or >1 missions).
- `derive_verdict` (`crates/codex1/src/state/readiness.rs:40-66`) ordering — `TerminalComplete` (close.terminal_at) → `NeedsUser` (!outcome) → `NeedsUser` (!plan.locked) → `Blocked` (replan.triggered) → `Blocked` (dirty review) → `ReadyForMissionCloseReview`/`Open`/`Passed` on `tasks_complete` → `ContinueRequired` — is used identically by `status` (`cli/status/project.rs:19`) and `close check` (`cli/close/check.rs:47`). Re-verified by the 11 unit tests at `state/readiness.rs::tests:149-296`. `02-cli-contract.md:208` ("status and close check must share readiness logic") holds.
- `EVENTS.jsonl` is append-only: `crates/codex1/src/state/events.rs:41-47` opens with `OpenOptions::new().create(true).append(true)`; `seq = state.events_cursor`, which is bumped exactly once per `state::mutate` call and never rewound. No code path writes historical events.
- Hard-planning evidence requirement enforced at `crates/codex1/src/cli/plan/check.rs:264-298` — rejects `effective: hard` plans without an evidence entry whose `kind` is in `HARD_EVIDENCE_KINDS = ["explorer","advisor","plan_review"]` (`crates/codex1/src/cli/plan/parsed.rs:112`).
- Six-consecutive-dirty replan trigger: `crates/codex1/src/cli/review/record.rs:28 DIRTY_STREAK_THRESHOLD = 6`; `apply_dirty:338-353` increments per target, `apply_clean:294-317` resets to 0. Mirrored for mission-close in `cli/close/record_review.rs:26,196-208`.
- Mission-close review is mandatory before `close complete`: `cli/close/complete.rs:41-46` refuses unless `ReadinessReport::ready`, which requires `Verdict::MissionCloseReviewPassed`, which requires `MissionCloseReviewState::Passed`. That state is only reachable via `cli/close/record_review.rs::record_clean:88-137`.

Round-1 fix verifications (all honored without regression):

- **`state::require_plan_locked` guards on work-phase commands.** Present and wired: `state/mod.rs:63-71` defines the helper; call sites at `cli/task/start.rs:22`, `cli/task/finish.rs:21`, `cli/review/start.rs:41`, `cli/review/record.rs:77` (terminal bypass is intentional so `TERMINAL_ALREADY_COMPLETE` wins precedence — see comment at lines 73-78). Unit tests pass (27 library tests green).
- **EVENTS.jsonl-before-STATE.json write order reversed.** `state/mod.rs:142-145` appends the event before `atomic_write(&state_path, …)`. Rationale documented at `state/mod.rs:126-141`. Crash window produces a detectable trailing JSONL line instead of a silent audit gap.
- **Parent-directory fsync after `persist`.** `state/fs_atomic.rs:33-35` calls `sync_all()` on the target's parent directory. macOS dir-fsync is a no-op (non-regressing); Linux/ext4/xfs now survive power-loss rename durability.
- **`outcome ratify` atomicity.** `cli/outcome/ratify.rs:60-75` runs `state::mutate` first, then `atomic_write(OUTCOME.md)` — any mutation-failure leaves OUTCOME.md unflipped. Rationale at `ratify.rs:54-59`.
- **`--expect-revision` enforcement on short-circuits.** `state::check_expected_revision` (`state/mod.rs:40-53`) is invoked in every idempotent / dry-run branch: `task/start.rs:49,78`, `task/finish.rs:74`, `plan/check.rs:78`, `plan/choose_level.rs:53`, `review/record.rs:101`, `close/complete.rs:53`, `close/record_review.rs:95,154`, `outcome/ratify.rs:34`, `loop_/mod.rs:75,79,92`.
- **Skill model matrix aligned.** `.codex/skills/review-loop/SKILL.md:53-59` and `.codex/skills/review-loop/references/reviewer-profiles.md:68,88` list `code_bug_correctness → gpt-5.3-codex` and `local_spec_intent → gpt-5.4`, matching handoff `04-roles-models-prompts.md:21-22,163-167`. No drift in `plan/references/hard-planning-evidence.md:11,63,115`. `execute/SKILL.md:84-86` preserves `gpt-5.3-codex` as the default coding worker with a Claude-family peer comment (Round-1 REJECT noted this as style-only).

No new P0/P1/P2 findings. The one P3 item below is low-priority polish, explicitly out-of-loop scope per decisions.md rule 5 but documented for completeness.

## P0

None.

## P1

None.

## P2

None.

## P3

### F1 — `ParsedPlan` does not `deny_unknown_fields`, so a user `waves:` section in PLAN.yaml is silently ignored

Citation: handoff `03-planning-artifacts.md:402-440` ("Do not store waves in `PLAN.yaml`... the source of truth is tasks + depends_on + current task state") and `README.md:72` ("Waves are derived from the DAG; waves are not stored as editable truth") vs `crates/codex1/src/cli/plan/parsed.rs:10-22`.

Evidence: `ParsedPlan` is `#[derive(Deserialize)]` with no `#[serde(deny_unknown_fields)]` attribute:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ParsedPlan {
    pub mission_id: Option<String>,
    pub planning_level: Option<PlanningLevel>,
    ...
    #[serde(default)]
    pub tasks: Vec<TaskSpec>,
    ...
}
```

Serde's default behavior ignores unknown top-level keys. A user authoring PLAN.yaml can add a `waves:` section, and `plan check` accepts the plan without flagging the drift. Because `ParsedPlan` has no `waves` field and downstream derivation reads `depends_on`, the ignored data never becomes "stored wave truth" inside the CLI — the anti-goal at `00-why-and-lessons.md:82-87` is technically honored. However, this is a UX footgun: a user can edit `waves:` expecting it to matter, see `plan check` return `ok: true`, and get surprised when `codex1 plan waves` ignores their edits.

Reproducer:

```yaml
# PLAN.yaml (snippet)
waves:
  - id: W0
    tasks: [T1]
tasks:
  - id: T1
    title: "..."
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
```

`codex1 plan check --json` returns `ok: true`; `codex1 plan waves --json` emits the derived wave `W1`, not the user's `W0`. Users and skills have no warning that their YAML was dropped.

Severity: P3 (anti-goal is honored at the storage layer; only UX-level). Not loop-scope per decisions.md rule 5.

Suggested fix (optional, out-of-scope): add `#[serde(deny_unknown_fields)]` to `ParsedPlan`, or emit a `plan check` warning when the raw YAML carries a `waves:` key.
