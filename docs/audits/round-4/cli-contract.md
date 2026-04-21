# Round 4 — cli-contract audit

**Commit audited:** 703a171
**Lens:** cli-contract
**Reviewer:** Opus 4.7

## Summary

Walked every command in `docs/codex1-rebuild-handoff/02-cli-contract.md` against `crates/codex1/src/cli/**/*.rs`. Built the binary with `make install-local` (installs to `~/.local/bin/codex1`), exercised every minimal-surface command with `--json` against a fresh temp mission in `/tmp/codex1-round4-test`, took a mission all the way from `init` through `outcome ratify` → `plan choose-level/scaffold/check` → `task start/finish` → `review start/packet/record/status` → `close record-review/check/complete` to `verdict: terminal_complete`. Also exercised the four loop transitions (`activate/pause/resume/deactivate`), both `replan` subcommands (including `--reason unknown` rejection and `--dry-run`), `plan graph` in all three formats plus `--out`, `status` error handling (unknown mission → `MISSION_NOT_FOUND`), and `REVISION_CONFLICT` on `--expect-revision 999`. `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` both pass clean on HEAD.

Round 3 found 0 P0/P1/P2 under this lens. Round 4 found **one new P2** not previously audited: `codex1 review packet` leaks the raw YAML block-scalar indicator (`|`) into the `mission_summary` field because `read_interpreted_destination` in `review/packet.rs` does substring YAML parsing (rather than proper `serde_yaml`) and a leading-whitespace bug lets the `|` token through. Reviewer subagents see a polluted mission summary as the first line of their prompt context. Accepted-deviation items (`data`-wrapped envelope; `complete` vs `terminal_complete` verdict token; `doctor` + `hook snippet` as documented extras) remain closed by round-1/2/3 decisions.

## P0

None.

## P1

None.

## P2

### P2-1 · `codex1 review packet` leaks YAML block-scalar indicator `|` into `mission_summary`

**Severity:** P2 (correctness-adjacent — reviewer packets are the literal prompt handed to reviewer subagents; a polluted first line is not state-breaking but wastes prompt context and confuses the reviewer role-prompt boundary).

**Finding validity:** (a) handoff vs divergent code + (b) reproducible bug.

**Handoff reference.** The reviewer packet is the mechanism described at `02-cli-contract.md:413-415`: "`codex1 review packet T4 --json` · Returns a reviewer packet the main thread can paste into reviewer prompts." Reviewer packets are consumed verbatim by a subagent.

**Code.** `crates/codex1/src/cli/review/packet.rs:136-176` implements `read_interpreted_destination` via substring + line-based scanning of the OUTCOME.md frontmatter. The relevant loop is at lines 146-168:

```rust
for line in rest.lines() {
    let trimmed = line.trim_end();          // only trim_end — leading ws survives
    if trimmed.is_empty() { ... continue; }
    if first_non_empty_seen { ... } else {
        first_non_empty_seen = true;
        if trimmed == "|" || trimmed == ">" { continue; }   // (A)
    }
    out.push_str(trimmed.trim_start());     // (B) — leaks content-sans-ws
    out.push('\n');
}
```

After `let start = raw.find("interpreted_destination:")?`, `rest` starts with the text `" |\n  Verify every command..."` (note the leading space between the colon and the `|`). The first line of `rest.lines()` is `" |"`. `trim_end()` leaves leading whitespace intact, so `trimmed` is `" |"` — not `"|"`. Branch (A) does not fire. The loop falls through to (B), which pushes `"|"` (after `trim_start`) into `out`. The body line `"  Verify every command..."` is appended after. The caller serializes the result as `mission_summary`.

**Reproduction (live CLI, commit 703a171):**

```
$ cd /tmp/codex1-round4-test && codex1 --json review packet T2 --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 7,
  "data": {
    ...
    "mission_summary": "|\nVerify every command and envelope matches the handoff spec.",
    ...
  }
}
```

The OUTCOME.md source is a standard `interpreted_destination: |\n  body…` block (no exotic YAML shape). Every newly-scaffolded OUTCOME.md follows this shape because `outcome emit`/`init` write it with the block-scalar indicator.

**Counter-examples in the same repo that do it right.** `crates/codex1/src/cli/task/worker_packet.rs:60-67` and `crates/codex1/src/cli/close/closeout.rs:136-140` both pull the same field using `serde_yaml::from_str` on the frontmatter, then `.trim()` the string — which yields the body without the `|` token:

```rust
// task/worker_packet.rs
let doc: serde_yaml::Value = serde_yaml::from_str(frontmatter).ok()?;
doc.get("interpreted_destination").and_then(|v| v.as_str()).map(|s| s.trim().to_string())
```

The correct field appears in `task packet` and `close complete`'s CLOSEOUT.md; only `review packet` uses the buggy substring parser.

**Why existing tests didn't catch it.** `tests/task.rs:480-484` asserts `summary.contains("mission used to exercise")` on the `task packet` path (which uses the correct parser). There is no equivalent test asserting `mission_summary` content on `review packet`, only structural checks.

**Suggested fix.** Replace `review/packet.rs::read_interpreted_destination` with the `serde_yaml`-based implementation already used at `task/worker_packet.rs:60-67` (either lift it to a shared helper in `core` or copy+trim). Either fixes the leak; the shared helper dedupes three copies. Add a regression test asserting `!mission_summary.starts_with('|')` and `!mission_summary.contains("|\n")`.

## P3

None new.

### Accepted deviations (already settled by round-1/2/3 decisions, re-affirmed)

- **Status `data`-wrapping.** Handoff example at `02-cli-contract.md:179-206` shows `phase`, `loop`, `next_action` at envelope top level; implementation wraps them in `data` via `core/envelope.rs:22-29`. Settled as accepted harness convention across rounds 1-3.
- **`complete` vs `terminal_complete` verdict token.** REJECTed in round-1 decisions (`00-why-and-lessons.md:173` canonical; handoff frozen per rule 5).
- **`doctor` and `hook snippet` commands outside the minimal surface.** Accepted in round-3 as documented additive utilities (doctor = install/verification health probe; hook snippet = Ralph wiring helper).

### Key-name divergence across `status.next_action` and `task next` (not flagged as a finding)

`cli/status/project.rs:194-199` emits `run_review` with field `review_task_id`; `cli/task/next.rs:67-70` emits `run_review` with field `task_id`. Both advertise the same review task. Handoff does not pin the exact field name for either command, so this is not a contract violation (rule 5 — handoff is frozen, and either name is consistent with the examples). Noting it for future convergence if skills start reading both; handoff line 208 requires `status` and `close check` to agree (they do), not `status` and `task next`. Not filed as a finding.

## Verification checklist (all passing)

- **Build / install.** `make install-local` succeeds at commit 703a171; installs `codex1` to `~/.local/bin/codex1`; `codex1 --help` and `codex1 --json doctor` both clean from `/tmp/codex1-round4-test`.
- **`cargo fmt --check`** clean (silent).
- **`cargo clippy --all-targets -- -D warnings`** clean (no warnings).
- **Envelope shape.** Success envelope at `core/envelope.rs:22-29` omits `mission_id`/`revision` when absent; error envelope at `core/envelope.rs:68-78` omits `hint`/`context` when empty. Stable across all commands tested.
- **Error codes.** All 14 suggested codes at `02-cli-contract.md:520-535` map to `CliError` variants in `core/error.rs:82-103`. Live-exercised `MISSION_NOT_FOUND` (unknown mission), `OUTCOME_INCOMPLETE` (fresh OUTCOME with fill markers), `OUTCOME_NOT_RATIFIED` (`plan choose-level` pre-ratify), `PLAN_INVALID` (unknown task kind, fill markers, bad replan reason, unknown supersedes), `REVISION_CONFLICT` (`--expect-revision 999`), `CLOSE_NOT_READY` (close complete before mission-close review), `PROOF_MISSING` path, `TASK_NOT_READY` paths.
- **Verdict values.** 7/8 match; `complete` vs `terminal_complete` is accepted deviation per prior rounds.
- **`plan choose-level` escalation payload.** Numeric `--level 1` → stored `light`. `--level 1 --escalate "touches hooks"` → `effective_level: hard`, `escalation_required: true`, `escalation_reason: "touches hooks"`, and `next_action.args` rewritten to `["codex1","plan","scaffold","--level","hard"]`. Matches `02-cli-contract.md:292-316`.
- **`plan graph` formats.** `--format mermaid|dot|json` all produce well-formed output. `--out <path>` writes the body to disk and returns `{"path": …}`.
- **`plan waves`** derives at call time; no `waves` truth in STATE (`state/schema.rs:178-194` has no waves field). Honors `02-cli-contract.md:118`.
- **`status` vs `close check` agreement.** At `verdict: terminal_complete`: `close check` emits `{verdict: terminal_complete, ready: false, blockers: []}`; `status` emits `{verdict: terminal_complete, close_ready: false, stop.reason: "terminal"}`. Both consume the same `state::readiness::derive_verdict`/`close_ready` helpers.
- **`close record-review` Open→Passed transition.** `crates/codex1/src/cli/close/record_review.rs` is the only write path that sets `close.review_state = Passed`; demonstrated live (`{ review_state: "passed" }` only after `close record-review --clean`). Matches `02-cli-contract.md:100-104`.
- **`--mission`/`--repo-root`/`--json`/`--dry-run`/`--expect-revision` surface.** All five flags are global on `Cli` (`cli/mod.rs:44-66`). Help text reflects them on every subcommand.
- **Review record freshness.** Four categories (`accepted_current`, `late_same_boundary`, `stale_superseded`, `contaminated_after_terminal`) all present in `review/classify.rs` and emitted in `review record --json` under `data.category`. Matches `02-cli-contract.md:445-450`.
- **Replan reason codes.** `six_dirty`, `scope_change`, `architecture_shift`, `risk_discovered`, `user_request` enforced at `replan record --reason`; live-tested rejection of `--reason unknown` returns the canonical `PLAN_INVALID`.
- **Reviewer-profile matrix.** `code_bug_correctness`, `local_spec_intent`, `integration_intent`, `plan_quality`, `mission_close` all recognized; `review packet` emits `review_profile`.
- **Initial STATE.json.** `codex1 init` creates the required subset (`02-cli-contract.md:139-147`): `mission_id`, `loop {active:false, paused:false, mode:"none"}`, `tasks:{}`, `reviews:{}`, `phase:"clarify"`. Extra fields (`revision`, `outcome`, `plan`, `replan`, `close`, `schema_version`, `events_cursor`) are additive.
- **`--proof` semantics.** Relative paths resolved against `mission_dir` at `cli/task/finish.rs:33-42`; live-tested with `--proof specs/T1/PROOF.md`.
- **Minimal-surface parity.** All 27 handoff-minimal commands resolve in `Commands` at `cli/mod.rs:73-125`. Two documented extras (`doctor`, `hook snippet`) accepted per round-3.
