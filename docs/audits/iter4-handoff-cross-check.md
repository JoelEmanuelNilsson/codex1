# Handoff Cross-Check — iter 4

Branch audited: `main` @ `5a16894`
Audited on: 2026-04-20 UTC.
Worktree: `.claude/worktrees/agent-a23c02b0` (no source, skill, script, or doc edits).

## Scope

Cross-check the repository at `5a16894` against every "Non-Negotiable" and "Anti-Goal" declared in:

- `docs/codex1-rebuild-handoff/00-why-and-lessons.md` § What To Reject.
- `docs/codex1-rebuild-handoff/README.md` § Non-Negotiables and § Explicit Anti-Goals.
- `docs/codex1-rebuild-handoff/02-cli-contract.md` § Important Non-Features.

For each anti-goal I confirm whether it is honored in live code, skills, scripts, and documentation. The iter 4 audit adds these specific greps from the task prompt:

- `.ralph/` only in anti-goal prose.
- No caller-identity patterns in `crates/codex1/src/**/*.rs` (`is_parent|is_subagent|caller_type|reviewer_id|session_id|session_token|capability_token|authority_token`).
- `plan scaffold` emits no `waves:` key.
- Reviewer writeback is positively forbidden in the `$review-loop` skill.
- No wrapper runtime / daemon / `Command::new("codex")` / `Command::new("claude")`.
- Ralph hook runs only `codex1 status --json`.

## Summary

**0 P0, 0 P1, 0 P2.**

Every declared anti-goal is honored at `5a16894`. `.ralph/` appears only as anti-goal prose or explicit prohibition; caller-identity patterns do not exist in Rust source; `plan scaffold` emits no `waves:` key in PLAN.yaml output; reviewer writeback is positively forbidden in the `$review-loop` skill; no wrapper runtime or background daemon is invoked by any Rust code; the Ralph hook runs only `codex1 status --json`; no capability / session / authority token shape exists anywhere.

## Build evidence

| Command | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test --release` | 169 passed / 0 failed / 0 ignored |

## Clean checks (anti-goal by anti-goal)

### CC-1: No `.ralph/` live directory, path, or import

Full grep for `.ralph` at `5a16894`:

| File | Line | Category |
| --- | --- | --- |
| `docs/codex1-rebuild-handoff/00-why-and-lessons.md` | 37, 82 | anti-goal prose |
| `docs/codex1-rebuild-handoff/01-product-flow.md` | 241 | anti-goal prose |
| `docs/codex1-rebuild-handoff/02-cli-contract.md` | 105, 117 | anti-goal prose |
| `docs/codex1-rebuild-handoff/03-planning-artifacts.md` | 28, 31, 160 | anti-goal prose |
| `docs/codex1-rebuild-handoff/05-build-prompt.md` | 36, 91 | anti-goal prose |
| `docs/codex1-rebuild-handoff/README.md` | 79, 93 | anti-goal prose |
| `docs/mission-anatomy.md` | 3 | "There is no hidden state directory, no `.ralph/`, no side-cache." |
| `README.md` | 3 | "There is no hidden `.ralph/` state…" |
| `scripts/README-hook.md` | 19 | "The hook does **not** read … `.ralph/` directories" |
| `.codex/skills/close/SKILL.md` | 112 | prohibition: "Do not edit `.ralph/` files, `STATE.json`, or hooks to work around Ralph." |
| `crates/codex1/src/lib.rs` | 9 | doc comment: "derived truth (waves), and never hides state in `.ralph/`." |
| `docs/audits/*` | — | prior audit reports; anti-goal prose |

No Rust source file, `scripts/*.sh`, skill body, or reference opens, reads, writes, or `Path::new`s a `.ralph/*` path. No `include_str!`/`include_bytes!`/`std::fs` call references it.

### CC-2: No caller-identity patterns in Rust source

Grep across `crates/codex1/src/**/*.rs` for the full pattern list in the task prompt:

```text
is_parent | is_subagent | caller_type | reviewer_id | session_id
session_token | capability_token | authority_token
```

**Zero matches** in any `.rs` file.

`crates/codex1/src/cli/mod.rs:124-140` defines `Ctx { mission, repo_root, json, dry_run, expect_revision }`; no caller-side identity is extracted or inspected. Every handler accepts the `Ctx` as-is and dispatches based on arguments + state-file contents. `crates/codex1/src/cli/review/mod.rs:2-4` makes the design explicit:

> The CLI does not check caller identity; the main thread records review outcomes.

### CC-3: `plan scaffold` emits no `waves:` key

- `crates/codex1/src/cli/plan/scaffold.rs::render_skeleton` (lines 119-171) composes the skeleton. The emitted YAML contains `mission_id`, `planning_level`, `outcome_interpretation`, `architecture`, `planning_process`, `tasks`, `risks`, and `mission_close` — no `waves:` key.
- Grep for `waves:` in `crates/codex1/src/cli/plan/scaffold.rs`: no matches.
- Grep for `waves:` in `crates/codex1/src/cli/init.rs`: no matches (the `init` template only adds `planning_level`, `outcome_interpretation`, `architecture`, `planning_process`, `tasks`, `risks`, `mission_close`).
- `crates/codex1/src/state/schema.rs::MissionState` (lines 176-194) has no `waves` field: `{ mission_id, revision, schema_version, phase, loop_, outcome, plan, tasks, reviews, replan, close, events_cursor }`.
- Empirical check: `target/release/codex1 init --mission m1` followed by `plan choose-level --level medium` and `plan scaffold --level medium` writes a `PLAN.yaml` with zero `waves:` occurrences.
- `codex1 plan waves --json` (`cli/plan/waves.rs:60-111`) derives waves on demand from `tasks[].depends_on` + current state; its output JSON contains a `"waves"` key, but that is a derived projection, not a stored `waves:` field. The command is read-only (no `state::mutate`, `fs::write`, or `atomic_write` in the file).

### CC-4: Reviewer writeback is positively forbidden in `$review-loop`

Three positive-worded prohibitions in the skill body:

- `.codex/skills/review-loop/SKILL.md:11`:
  > The main thread is the sole writer of mission truth: it records clean/dirty via `codex1 review record` (planned review) or `codex1 close record-review` (mission close).
- `.codex/skills/review-loop/SKILL.md:63`:
  > Every spawn prompt must begin with the standing reviewer block in `references/reviewer-profiles.md` (findings-only, no edits, no `codex1` mutations, no repairs, no marking clean anywhere). The main thread is the sole writer of mission truth. Reviewer writeback is forbidden.
- `.codex/skills/review-loop/SKILL.md:79`:
  > Record review results from inside a reviewer subagent — only the main thread records.

The standing reviewer block in `.codex/skills/review-loop/references/reviewer-profiles.md:8-15` repeats the prohibition in the prompt each reviewer receives:

```text
Do not edit files.
Do not invoke Codex1 skills.
Do not run codex1 mutating commands (no `review record`, `task finish`, `close *`, etc.).
Do not record mission truth.
```

`.codex/skills/execute/SKILL.md:117` reinforces from the orchestrator side: "Do not spawn reviewers, run `codex1 review record`, or write to `reviews/`."

### CC-5: No wrapper runtime / daemon / `Command::new("codex")` / `Command::new("claude")`

- Grep for `Command::new` inside `crates/codex1/src`: **0 matches**.
- Grep for `daemon`, `spawn_process`, `wrapper_runtime`, `tokio::spawn`: **0 matches**.
- `crates/codex1/src/cli/doctor.rs:63-72` probes for a `codex1` binary on `PATH` via `std::env::split_paths` + `candidate.is_file()` (a file-existence check, not an invocation).
- `scripts/ralph-stop-hook.sh:25` invokes `$CODEX1 status --json` exactly once and exits; no background process is spawned. The hook drains stdin, probes for the binary, runs one command, parses `.data.stop.allow`, and exits 0 or 2.

The Codex1 binary is a one-shot CLI on every invocation. Nothing in the repo starts, manages, or talks to a long-running helper process.

### CC-6: Ralph hook runs only `codex1 status --json`

Full audit of `scripts/ralph-stop-hook.sh` (60 lines):

| Line | Behavior |
| --- | --- |
| 14 | Drains stdin so the pipe does not stall. |
| 16 | `CODEX1=${CODEX1_BIN:-codex1}` — respects `$CODEX1_BIN` override. |
| 18-21 | If `codex1` not on PATH, allow Stop (exit 0). |
| 25 | `status_json="$("$CODEX1" status --json 2>/dev/null || true)"` — the sole CLI invocation. |
| 36-45 | Parses `.data.stop.allow` / `.reason` / `.message` with `jq`, or a degraded grep fallback. |
| 47-60 | Exit 0 on `allow=true`, exit 2 on `allow=false` (blocks Stop), exit 0 on parse failure (fail-open). |

The hook never reads `PLAN.yaml`, `STATE.json`, `EVENTS.jsonl`, `specs/`, `reviews/`, or `.ralph/`. It never spawns another process. `scripts/README-hook.md:19` makes the promise explicit.

`crates/codex1/src/cli/hook.rs` only prints wiring JSON (`codex1 hook snippet`); it does not install or invoke anything.

### CC-7: No capability / session / authority tokens

Grep across `crates/codex1/src` for `session_id | session_token | capability_token | authority_token | session_owner`: **0 matches**.

No handler accepts, mints, stores, or validates an authority token. Mutating commands gate on `--expect-revision` (`core/error.rs:53-54`), which is a state-file revision counter, not a caller-identity credential.

### CC-8: Stored waves are not treated as editable truth

- `crates/codex1/src/state/schema.rs::MissionState` (lines 176-194) has no `waves` field.
- `crates/codex1/src/cli/plan/waves.rs` derives waves on demand; no `state::mutate` call inside the file.
- `docs/cli-contract-schemas.md` pins the Rust types for `MissionState`; no `waves:` collection is listed.
- `.codex/skills/plan/SKILL.md:199` positively forbids storing waves: "Do not store waves inside `PLAN.yaml`. Waves are derived."

### CC-9: CLI does not spawn subagents or ask semantic questions

- `crates/codex1/src/cli/**/*.rs` contains no `Command::new("codex")` / `Command::new("claude")` / model invocation.
- No handler reads from stdin except `cli/plan/choose_level.rs:110` (TTY-only level prompt), which is the explicitly sanctioned exception at `docs/codex1-rebuild-handoff/02-cli-contract.md:46`.
- `codex1 task packet` (`cli/task/packet.rs`) and `codex1 review packet` (`cli/review/packet.rs`) emit prompt strings for the main thread to paste into a subagent; neither spawns anything.

### CC-10: Visible mission files only — no hidden state

`crates/codex1/src/core/paths.rs` resolves every mutating command to `PLANS/<mission>/{OUTCOME.md, PLAN.yaml, STATE.json, EVENTS.jsonl, specs/, reviews/, CLOSEOUT.md}`. No handler writes outside `PLANS/<mission>/`. There is no side-cache, side-state, or hidden dotfile.

### CC-11: `plan choose-level` is the only interactive prompt, and it asks a structured question

`crates/codex1/src/cli/plan/choose_level.rs:110` (interactive TTY branch) asks exactly one question: which of `light | medium | hard | 1 | 2 | 3` to record. It does not ask semantic clarification about the mission. This is the sanctioned exception at `02-cli-contract.md:46`. All other commands are non-interactive and emit JSON.

## Reading map — task bullets versus this audit

| Task iter 4 scope line | Verified at |
| --- | --- |
| `.ralph/` only in anti-goal prose | CC-1 |
| No caller-identity patterns in `crates/codex1/src/**/*.rs` | CC-2 |
| `plan scaffold` emits no `waves:` key | CC-3 |
| Reviewer writeback positively forbidden in `$review-loop` | CC-4 |
| No wrapper runtime / daemon / `Command::new("codex"\|"claude")` | CC-5 |
| Ralph hook runs only `codex1 status --json` | CC-6 |

| Handoff source | Anti-goal | Verified at |
| --- | --- | --- |
| `00-why-and-lessons.md:82` | No `.ralph` mission truth | CC-1 |
| `00-why-and-lessons.md:84` | No caller-identity checks | CC-2 |
| `00-why-and-lessons.md:86` | No reviewer writeback authority | CC-4 |
| `00-why-and-lessons.md:87` | No stored waves as editable truth | CC-3, CC-8 |
| `00-why-and-lessons.md:80-81` | No hidden daemons / wrapper runtimes | CC-5 |
| `README.md:78` | CLI must not detect parent vs subagent | CC-2 |
| `README.md:79` | No `.ralph/` mission state directory | CC-1 |
| `README.md:89` | No session-ID authority system | CC-7 |
| `README.md:91` | No capability-token maze | CC-7 |
| `README.md:92` | No reviewer writeback authority tokens | CC-4, CC-7 |
| `README.md:93` | No stored waves as canonical truth | CC-3, CC-8 |
| `README.md:95` | A CLI that spawns subagents (rejected) | CC-5, CC-9 |
| `README.md:96` | A CLI that asks semantic clarification (rejected) | CC-11 |
| `02-cli-contract.md:99` | Must not ask "are you parent or subagent?" | CC-2 |
| `02-cli-contract.md:102` | Must not spawn subagents | CC-9 |
| `02-cli-contract.md:105` | Must not use `.ralph` mission truth | CC-1 |
| `02-cli-contract.md:106` | Must not store waves as editable truth | CC-3, CC-8 |
| `01-product-flow.md:241` | Ralph must not inspect plan/review files directly | CC-6 |

Every anti-goal is honored in code, skills, scripts, and docs. No findings.
