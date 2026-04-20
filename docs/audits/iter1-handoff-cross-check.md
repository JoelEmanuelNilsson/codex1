# Handoff Cross-Check — iter1 (post-6473650)

Branch audited: `audit-iter1-wave` (off `main`) @ `6473650`
Audited on: 2026-04-20 UTC.

## Build / test evidence (iter1 header)

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets -- -D warnings` | FAIL (see `iter1-cli-contract-audit.md` P2-1; not a handoff issue) |
| `cargo test --release` | PASS — 169 tests across 19 test binaries (steady-state). Cold-cache flake documented in the CLI audit. |

Clippy failure is in a test file, not a handoff artifact; reported in the CLI contract audit.

## Scope

Cross-checks every "Non-Negotiable" and "Anti-Goal" declared in:

- `docs/codex1-rebuild-handoff/00-why-and-lessons.md` § What To Reject.
- `docs/codex1-rebuild-handoff/README.md` § Non-Negotiables and § Explicit Anti-Goals.
- `docs/codex1-rebuild-handoff/02-cli-contract.md` § Important Non-Features.

For each anti-goal I verified whether it is honored in live code, skills, scripts, and docs.

## Summary

**0 P0, 0 P1, 0 P2.** Every declared anti-goal is still honored post-6473650. `.ralph/` only appears as an explicit prohibition in documentation, scripts, and skill prose; caller-identity patterns do not exist in Rust source; `plan scaffold` does not emit a `waves:` key; reviewer writeback is positively forbidden in `$review-loop`; no wrapper runtime, daemon, or subagent-spawning process is invoked by the CLI; Ralph's Stop hook only calls `codex1 status --json`.

## Findings

None.

## Clean checks (anti-goal by anti-goal)

### CC-1: No `.ralph/` live directory, path, or import

- `/Users/joel/codex1/.ralph/` does not exist as a filesystem entry (`ls -la` returns `No such file or directory`).
- All 20-ish matches for the substring `.ralph` in the repo (excluding `target/**`) are either (a) inside handoff docs where `.ralph` is defined as an anti-goal, (b) inline prohibitions in skill prose, (c) assertions in iter1 / baseline audit files, or (d) a doc-comment in `crates/codex1/src/lib.rs:9`. Key live-code/script citations:

| File | Line(s) | Kind |
| --- | --- | --- |
| `crates/codex1/src/lib.rs` | 9 | Doc-comment ("never hides state in `.ralph/`.") |
| `.codex/skills/close/SKILL.md` | 112 | Positive prohibition |
| `scripts/README-hook.md` | 19 | Hook scope ("hook does not read `.ralph/` directories") |
| `README.md` | 3 | Anti-goal prose |
| `docs/mission-anatomy.md` | 3 | Anti-goal prose |
| `docs/codex1-rebuild-handoff/**` | multiple | Anti-goal doc lines |

No `std::fs`, `Path::new`, `PathBuf`, `include_str!`, `include_bytes!`, or shell invocation in the repo references a `.ralph` path.

### CC-2: No caller-identity checks in Rust source

Grep over `crates/codex1/src/**/*.rs` for:

```text
is_parent | is_subagent | caller_type | reviewer_id | session_id |
session_token | capability_token | authority_token
```

**0 matches** across all Rust source.

`crates/codex1/src/cli/mod.rs:128-134` defines `Ctx` with only `{ mission, repo_root, json, dry_run, expect_revision }`; no caller-side identity is extracted or inspected. The comment at `crates/codex1/src/cli/review/mod.rs` is explicit: "The CLI does not check caller identity; the main thread records review outcomes."

### CC-3: No `waves:` key emitted by `plan scaffold`

- `crates/codex1/src/cli/plan/scaffold.rs:119-171` (`render_skeleton`) composes the PLAN.yaml skeleton. The output contains `mission_id`, `planning_level`, `outcome_interpretation`, `architecture`, `planning_process`, `tasks`, `risks`, and `mission_close` — no `waves:` key.
- Grep for `waves:` as a YAML key emission across the repo: 0 hits.
- `codex1 plan waves --json` (at `crates/codex1/src/cli/plan/waves.rs`) recomputes waves from `tasks[].depends_on` + state on each call. No persisted `waves:` collection is read from any artifact.
- `crates/codex1/src/state/schema.rs:170-188` defines `MissionState` with no `waves` field.

### CC-4: Reviewer writeback is positively forbidden

`$review-loop`'s skill body and references explicitly state that the main thread records, not the reviewer:

- `.codex/skills/review-loop/SKILL.md:11`: "The main thread is the sole writer of mission truth: it records clean/dirty via `codex1 review record` (planned review) or `codex1 close record-review` (mission close)."
- `.codex/skills/review-loop/SKILL.md:63`: "Every spawn prompt must begin with the standing reviewer block in `references/reviewer-profiles.md` (findings-only, no edits, no `codex1` mutations, no repairs, no marking clean anywhere). The main thread is the sole writer of mission truth. Reviewer writeback is forbidden."
- `.codex/skills/review-loop/SKILL.md:79`: "Record review results from inside a reviewer subagent — only the main thread records."
- `.codex/skills/review-loop/references/reviewer-profiles.md` (standing reviewer instructions): "Do not edit files. Do not invoke Codex1 skills. Do not run codex1 mutating commands (no `review record`, `task finish`, `close *`, etc.). Do not record mission truth."
- `.codex/skills/execute/SKILL.md:117`: "Do not spawn reviewers, run `codex1 review record`, or write to `reviews/`."

Positive and negative forms are both present — exactly what the handoff asks for.

### CC-5: No wrapper runtime / `codex1-runtime` / `ralph-daemon`

Grep over the repo for `codex1-runtime | ralph-daemon | wrapper_runtime`:

```bash
$ grep -rI --exclude-dir=target "codex1-runtime\|ralph-daemon\|wrapper_runtime" .
# (only matches are inside audit docs describing the anti-goal)
```

Grep for `Command::new("codex")` / `Command::new("claude")` / `spawn_process`:

- `crates/codex1/src/cli/doctor.rs` probes for a `codex1` binary on `PATH` (non-invocation — it only checks path existence).
- `scripts/ralph-stop-hook.sh:25` runs exactly one command: `"$CODEX1" status --json`.
- No `std::process::Command::spawn` / `tokio::spawn` is used to launch any background worker, daemon, or helper process.

The binary is a one-shot CLI on every invocation. No wrapper runtime exists.

### CC-6: CLI does not spawn subagents or read hidden chat state

- `crates/codex1/src/cli/**/*.rs` contains no `Command::new("codex")`, `Command::new("claude")`, or AI-model invocation.
- The only stdin consumer is `crates/codex1/src/cli/plan/choose_level.rs:102-120`, and only when stdin is a TTY (the documented interactive level prompt).
- `codex1 task packet` and `codex1 review packet` emit prompt strings for the main thread to paste into a subagent; neither spawns anything itself.

### CC-7: Ralph is status-only

`scripts/ralph-stop-hook.sh` (60 lines):

- Drains stdin.
- Probes for `codex1` on `PATH` (or via `$CODEX1_BIN`).
- Runs exactly one command: `codex1 status --json` (line 25).
- Parses `.data.stop.allow` with `jq` (or a degraded grep fallback).
- Exits 2 iff `allow == false`, else 0.

It never reads `PLAN.yaml`, `STATE.json`, `EVENTS.jsonl`, reviews, specs, or `.ralph/`. It never spawns another process. `scripts/README-hook.md:19-22` makes this promise explicit.

`codex1 hook snippet` (`crates/codex1/src/cli/hook.rs`) only prints wiring JSON; it does not install, probe, or invoke anything.

### CC-8: Stored waves are not treated as editable truth

- `crates/codex1/src/state/schema.rs:170-188` (`MissionState`) has no `waves` field.
- `crates/codex1/src/cli/plan/waves.rs` derives waves on demand from plan tasks + state.
- `docs/cli-contract-schemas.md:100-119` pins the Rust types for `MissionState`; no `waves:` collection is listed.

### CC-9: No capability tokens / authority tokens / session-id authority

Grep for `session_id | session_token | capability_token | authority_token` across `crates/codex1`: 0 matches.

No handler accepts, mints, stores, or validates any capability/authority token. Mutating commands gate on `--expect-revision` (a state-file revision counter), not on any caller-identity credential.

### CC-10: `docs/cli-contract-schemas.md` remains foundation-owned

The file's own declaration at `docs/cli-contract-schemas.md:369-385` pins it Foundation-owned. This audit does not modify it (nor any Rust source or existing doc).

Commit `6473650` legitimately expanded `cli-contract-schemas.md` to fix baseline P1-2, P2-2, F4, F5, F6 — those are Foundation-level changes documented in the commit message.

### CC-11: CLI does not ask semantic questions

`crates/codex1/src/cli/plan/choose_level.rs:102-120` is the only interactive prompt in the entire CLI. It asks a single question: which of `light | medium | hard` to record. It does not ask semantic clarification questions about the mission. Per `docs/codex1-rebuild-handoff/02-cli-contract.md:45-46` this is the explicitly sanctioned exception.

All other commands are non-interactive and emit JSON.

### CC-12: Visible mission files only

`crates/codex1/src/core/paths.rs` resolves `PLANS/<mission>/{OUTCOME.md, PLAN.yaml, STATE.json, EVENTS.jsonl, specs/, reviews/, CLOSEOUT.md}`. Every mutating command uses these paths; no handler writes outside `PLANS/<mission>/`. There is no side-cache, side-state, or hidden dotfile.

## Reading map

| Anti-goal (handoff source) | Verified at |
| --- | --- |
| `00-why-and-lessons.md:82` – "No `.ralph` mission truth." | CC-1 |
| `00-why-and-lessons.md:84` – "No caller identity checks." | CC-2 |
| `00-why-and-lessons.md:86` – "No reviewer writeback authority systems." | CC-4 |
| `00-why-and-lessons.md:87` – "No stored waves as editable truth." | CC-3, CC-8 |
| `00-why-and-lessons.md:80` – "No hidden daemons." | CC-5 |
| `00-why-and-lessons.md:81` – "No wrapper runtimes around Codex." | CC-5 |
| `README.md:79` – "No `.ralph/` mission state directory." | CC-1 |
| `README.md:78` – "CLI must not detect parent vs subagent." | CC-2 |
| `README.md:89` – "No session-ID authority system." | CC-9 |
| `README.md:91` – "No capability-token maze." | CC-9 |
| `README.md:92` – "No reviewer writeback authority tokens." | CC-4 |
| `README.md:93` – "No stored waves as canonical truth." | CC-3, CC-8 |
| `README.md:95` – "A CLI that spawns subagents." | CC-6 |
| `README.md:96` – "A CLI that asks semantic clarification questions." | CC-11 |
| `02-cli-contract.md:112` – "Must not ask 'are you parent or subagent?'" | CC-2 |
| `02-cli-contract.md:117` – "Must not use `.ralph` mission truth." | CC-1 |
| `02-cli-contract.md:118` – "Must not store waves as editable truth." | CC-3, CC-8 |
| `02-cli-contract.md:115` – "Must not spawn subagents." | CC-6 |
| `01-product-flow.md` – "Ralph must not inspect plan/review files directly." | CC-7 |

Every anti-goal was verified clean. No P0/P1/P2 findings.
