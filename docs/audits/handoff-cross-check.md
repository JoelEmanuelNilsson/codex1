# Handoff Cross-Check

Branch audited: `integration/phase-b` @ `a9e9abc`
Audited on: 2026-04-20 UTC.

## Scope

This audit cross-checks the repository against every "Non-Negotiable" and "Anti-Goal" declared in:

- `docs/codex1-rebuild-handoff/00-why-and-lessons.md` § What To Reject.
- `docs/codex1-rebuild-handoff/README.md` § Non-Negotiables and § Explicit Anti-Goals.
- `docs/codex1-rebuild-handoff/02-cli-contract.md` § Important Non-Features.

For each anti-goal I report whether it is honored in live code, skills, scripts, and docs.

## Summary

0 P0, 0 P1, 0 P2. Every declared anti-goal is honored. `.ralph/` only appears as an explicit prohibition in documentation and scripts; caller-identity patterns do not exist in Rust source; `plan scaffold` does not emit a `waves:` key; reviewer writeback is positively forbidden in the skill bodies; and no wrapper runtime or background daemon is invoked by the CLI.

## Findings

None.

## Clean checks (anti-goal by anti-goal)

### CC-1: No `.ralph/` live directory, path, or import

All 11 matches for the substring `.ralph` in the repository are either (a) inside handoff docs where `.ralph` is defined as an anti-goal or (b) inline prohibitions:

| File | Line | Kind |
| --- | --- | --- |
| `docs/codex1-rebuild-handoff/00-why-and-lessons.md` | 37, 82 | Anti-goal doc |
| `docs/codex1-rebuild-handoff/01-product-flow.md` | 241 | Anti-goal doc |
| `docs/codex1-rebuild-handoff/02-cli-contract.md` | 105 | Anti-goal doc |
| `docs/codex1-rebuild-handoff/03-planning-artifacts.md` | 28, 31 | Anti-goal doc ("Do not use `.ralph` for mission truth.") |
| `docs/codex1-rebuild-handoff/05-build-prompt.md` | 36, 91 | Anti-goal doc |
| `docs/codex1-rebuild-handoff/README.md` | 79, 93 | Anti-goal doc |
| `docs/mission-anatomy.md` | 3 | Anti-goal prose ("There is no hidden state directory, no `.ralph/`…") |
| `README.md` | 3 | Anti-goal prose ("There is no hidden `.ralph/` state…") |
| `scripts/README-hook.md` | 19 | Ralph hook scope ("The hook does **not** read … `.ralph/` directories") |
| `.codex/skills/close/SKILL.md` | 112 | Prohibition ("Do not edit `.ralph/` files, STATE.json, or hooks to work around Ralph.") |
| `crates/codex1/src/lib.rs` | 9 | Doc comment ("never hides state in `.ralph/`.") |

No `std::fs`, `Path::new`, `PathBuf`, `include_str!`, `include_bytes!`, or shell invocation in the repo references a `.ralph` path. Grep confirms no Rust source creates, reads, or writes anything under `.ralph/`.

### CC-2: No caller-identity checks in Rust source

Grep over `crates/codex1` for:

```text
is_parent | is_subagent | caller_type | reviewer_id | parent_id | subagent_type
session_owner | caller_identity | session_id | session_token | capability_token
authority_token
```

Zero matches in any `.rs` file. `crates/codex1/src/cli/mod.rs:124-140` defines `Ctx` with only `{ mission, repo_root, json, dry_run, expect_revision }`; no caller-side identity is extracted or inspected. Every handler accepts the `Ctx` as-is and dispatches based on arguments + state file contents.

The comment in `crates/codex1/src/cli/review/mod.rs:2-4` makes this explicit:

> The CLI does not check caller identity; the main thread records review
> outcomes.

### CC-3: No `waves:` key emitted by `plan scaffold`

- `crates/codex1/src/cli/plan/scaffold.rs:119-171` (`render_skeleton`) composes the PLAN.yaml skeleton. The output contains `mission_id`, `planning_level`, `outcome_interpretation`, `architecture`, `planning_process`, `tasks`, `risks`, and `mission_close` — and no `waves:` key.
- Grep for `waves:` / `wave_id` across `crates/codex1/src/cli/plan/` finds only `graph.rs:11` (`use super::waves::{load_plan_tasks, ParsedTask};`) — a module import, not a YAML key emission.
- Grep for `waves:` under `.codex/skills/` / `docs/` yields no emission of `waves:` as a plan field; every match is prose describing that waves are derived.
- `codex1 plan waves --json` recomputes waves from `tasks[].depends_on` + current task state on each call (`cli/plan/waves.rs`), never from a stored `waves:` list.

### CC-4: Reviewer writeback — main thread records (positive prohibition)

`$review-loop`'s skill body and references explicitly say the main thread records, not the reviewer:

- `.codex/skills/review-loop/SKILL.md:11`:
  > The main thread is the sole writer of mission truth: it records clean/dirty via `codex1 review record` (planned review) or `codex1 close record-review` (mission close).
- `.codex/skills/review-loop/SKILL.md:63`:
  > Every spawn prompt must begin with the standing reviewer block in `references/reviewer-profiles.md` (findings-only, no edits, no `codex1` mutations, no repairs, no marking clean anywhere). The main thread is the sole writer of mission truth. Reviewer writeback is forbidden.
- `.codex/skills/review-loop/SKILL.md:79`:
  > - Record review results from inside a reviewer subagent — only the main thread records.
- `.codex/skills/review-loop/references/reviewer-profiles.md:8-15` (standing reviewer instructions):
  > Do not edit files. / Do not invoke Codex1 skills. / Do not run codex1 mutating commands (no `review record`, `task finish`, `close *`, etc.). / Do not record mission truth.

`$execute/SKILL.md:117` reinforces from the orchestration side: `- Do not spawn reviewers, run codex1 review record, or write to reviews/`.

The handoff's required "main-thread records" language is present in both positive and negative forms — exactly what the task's positive-prohibition check asked for.

### CC-5: No wrapper runtime / `codex1-runtime` / `ralph-daemon`

Grep over the repo for `codex1-runtime | ralph-daemon | wrapper_runtime`:

```bash
$ grep -rI --exclude-dir=target "codex1-runtime\|ralph-daemon\|wrapper_runtime" .
# (no matches)
```

Grep for `daemon | spawn_process | Command::new.*codex` in source:

- `crates/codex1/src/cli/doctor.rs:64-66` probes for a `codex1` binary on `PATH` (non-invocation).
- `scripts/ralph-stop-hook.sh:25` runs `codex1 status --json` once and exits — no background process.

No `std::process::Command::spawn` / `tokio::spawn` is used to launch any background worker, daemon, or helper process. The binary is a one-shot CLI on every invocation.

### CC-6: CLI does not spawn subagents or read hidden chat state

- `crates/codex1/src/cli/**/*.rs` contains no `Command::new("codex")` / `Command::new("claude")` / AI-model invocation.
- No handler reads from stdin except `plan choose-level` (`cli/plan/choose_level.rs:102-120`), and only when stdin is a TTY — i.e. for the documented interactive level prompt.
- `codex1 task packet` and `codex1 review packet` emit prompt strings for the main thread to paste into a subagent; neither spawns anything.

### CC-7: Ralph is status-only

`scripts/ralph-stop-hook.sh` (60 lines, owned by Unit 12):

- Drains stdin.
- Probes for `codex1` on `PATH` (or via `$CODEX1_BIN`).
- Runs exactly one command: `codex1 status --json` (line 25).
- Parses `.data.stop.allow` with `jq` (or a degraded grep fallback).
- Exits 2 iff `allow == false`, else 0.

It never reads PLAN.yaml, STATE.json, EVENTS.jsonl, reviews, specs, or `.ralph/`. It never spawns another process. `scripts/README-hook.md:19-22` makes this promise explicit.

`codex1 hook snippet` (`crates/codex1/src/cli/hook.rs:24-50`) only prints wiring JSON; it does not install, probe, or invoke anything.

### CC-8: Stored waves are not treated as editable truth

- `crates/codex1/src/state/schema.rs:171-188` defines `MissionState`. The struct contains `tasks`, `reviews`, `replan`, `close`, etc. — no `waves` field.
- `crates/codex1/src/cli/plan/waves.rs` derives waves on demand from plan tasks + state.
- `docs/cli-contract-schemas.md:78-97` pins the Rust types for `MissionState`; no `waves:` collection is listed.

### CC-9: Capability tokens / authority tokens / session-id authority

Grep for `session_id | session_token | capability_token | authority_token` across `crates/codex1`: 0 matches.

No handler accepts, mints, stores, or validates a capability/authority token. Mutating commands gate on `--expect-revision` (`core/error.rs:53-54`), which is a state-file revision counter, not a caller-identity credential.

### CC-10: `docs/cli-contract-schemas.md` remains foundation-owned and untouched

Per the contract at `docs/cli-contract-schemas.md:298-325`, `cli-contract-schemas.md` is Foundation-owned. This audit does not modify that file (nor any Rust source). `cargo build --release` on the audited tree succeeds unchanged.

### CC-11: CLI does not ask semantic questions

`crates/codex1/src/cli/plan/choose_level.rs:102-120` is the only interactive prompt in the entire CLI. It asks a single question: which of `light | medium | hard` to record. It does not ask semantic clarification questions about the mission; per `docs/codex1-rebuild-handoff/02-cli-contract.md:46` this is the explicitly sanctioned exception.

All other commands are non-interactive and emit JSON.

### CC-12: Visible mission files only

`crates/codex1/src/core/paths.rs` resolves `PLANS/<mission>/{OUTCOME.md, PLAN.yaml, STATE.json, EVENTS.jsonl, specs/, reviews/, CLOSEOUT.md}`. Every mutating command uses these paths; no handler writes outside `PLANS/<mission>/`. There is no side-cache, side-state, or hidden dotfile.

## Reading map

Each anti-goal and the line where I verified it:

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
| `02-cli-contract.md:99` – "Must not ask 'are you parent or subagent?'" | CC-2 |
| `02-cli-contract.md:105` – "Must not use `.ralph` mission truth." | CC-1 |
| `02-cli-contract.md:106` – "Must not store waves as editable truth." | CC-3, CC-8 |
| `02-cli-contract.md:102` – "Must not spawn subagents." | CC-6 |
| `01-product-flow.md:241` – "Ralph must not inspect plan/review files directly." | CC-7 |

Every anti-goal was verified clean. No P0/P1/P2 findings.
