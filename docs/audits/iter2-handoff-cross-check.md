# Iter2 Handoff Cross-Check

Branch audited: `audit-iter2-wave` (worktree off `main` @ `271b2fc`).
Audit iteration: iter 2 (after iter1 fix at `6473650` and the clippy follow-up at `271b2fc`).
Audited on: 2026-04-20 UTC.

## Build evidence

| Command | Result |
| --- | --- |
| `cargo fmt --check` | PASS (no diff) |
| `cargo clippy --all-targets -- -D warnings` | PASS (zero warnings) |
| `cargo test --release` | PASS — 169 passed / 0 failed / 0 ignored across 18 binaries |

## Scope

Re-ran every anti-goal check named in the handoff against commit `271b2fc`:

1. No `.ralph/` as a live path / import / fs call.
2. No caller-identity patterns in Rust source (`is_parent`, `is_subagent`, `caller_type`, `reviewer_id`, `session_id`, `session_token`, `capability_token`, `authority_token`, `parent_id`, `subagent_type`).
3. `plan scaffold` does not emit `waves:` in the rendered PLAN.yaml skeleton.
4. Reviewer writeback positively forbidden in skill + references.
5. No wrapper runtime / daemon / `Command::new("codex")` / `Command::new("claude")`.
6. Ralph hook is status-only (one `codex1 status --json` call, no other invocation).

Sources audited: `crates/codex1/src/**`, `crates/codex1/tests/**`, `.codex/skills/**`, `scripts/**`, `docs/**`, `Cargo.toml`, `Makefile`, `README.md`.

## Summary

**0 P0 / 0 P1 / 0 P2.** Tree is clean. Every declared anti-goal is honored.

## Findings

No findings.

## Clean checks (anti-goal by anti-goal)

### CC-1: No `.ralph/` live directory, path, import, or fs call

Grep over the entire repo for the substring `.ralph` — every match is either a handoff anti-goal definition or an inline prohibition. No live use:

| File | Line(s) | Kind |
| --- | --- | --- |
| `docs/codex1-rebuild-handoff/00-why-and-lessons.md` | 82 | Anti-goal doc ("`.ralph` as mission truth.") |
| `docs/codex1-rebuild-handoff/01-product-flow.md` | 241 | Anti-goal doc ("Ralph must not use `.ralph` mission truth.") |
| `docs/codex1-rebuild-handoff/02-cli-contract.md` | 117 | Anti-goal doc ("Use `.ralph` mission truth.") |
| `docs/codex1-rebuild-handoff/03-planning-artifacts.md` | 28, 160 | Anti-goal doc ("Do not use `.ralph` for mission truth.", "Do not use .ralph as mission state.") |
| `docs/codex1-rebuild-handoff/05-build-prompt.md` | 36, 91 | Anti-goal doc |
| `docs/codex1-rebuild-handoff/README.md` | 79 | Anti-goal doc ("No `.ralph` mission state directory.") |
| `docs/mission-anatomy.md` | 3 | Anti-goal prose ("There is no hidden state directory, no `.ralph/`, no side-cache.") |
| `README.md` | 3 | Anti-goal prose |
| `scripts/README-hook.md` | 19 | Hook scope ("The hook does **not** read … `.ralph/` directories") |
| `.codex/skills/close/SKILL.md` | 112 | Prohibition ("Do not edit `.ralph/` files, `STATE.json`, or hooks to work around Ralph.") |
| `crates/codex1/src/lib.rs` | 9 | Doc comment ("never hides state in `.ralph/`.") |
| `docs/audits/handoff-cross-check.md` | (multiple) | Iter1 audit prose |
| `docs/audits/skills-audit.md` | (multiple) | Iter1 audit prose |

No `std::fs::*`, `Path::new("..ralph..")`, `PathBuf::from("..ralph..")`, `include_str!`, `include_bytes!`, `Command::new` invocation references a `.ralph` path. The string never appears as an argument, only as text inside doc comments and prohibitions.

### CC-2: No caller-identity patterns in Rust source

Grep over `crates/` for the regex
`is_parent|is_subagent|caller_type|reviewer_id|parent_id|subagent_type|session_owner|caller_identity|session_id|session_token|capability_token|authority_token`:

```
(no matches)
```

Zero hits in any `.rs` file.

`crates/codex1/src/cli/mod.rs:128-134` defines the request `Ctx`:

```rust
pub struct Ctx {
    pub mission: Option<String>,
    pub repo_root: Option<PathBuf>,
    pub json: bool,
    pub dry_run: bool,
    pub expect_revision: Option<u64>,
}
```

No identity field, no session token, no capability extraction. Every handler accepts the `Ctx` as-is and dispatches on arguments + state file contents only.

The `crates/codex1/src/cli/review/mod.rs:2-4` doc comment makes the intent explicit:

> The CLI does not check caller identity; the main thread records review outcomes.

### CC-3: `plan scaffold` does not emit `waves:`

`crates/codex1/src/cli/plan/scaffold.rs:119-171` (`render_skeleton`) is the single entry point that writes PLAN.yaml. The composed string contains:

```text
mission_id: <id>
planning_level: { requested, effective }
outcome_interpretation: { summary }
architecture: { summary, key_decisions }
planning_process: { evidence }
tasks: []
risks: [{ risk, mitigation }]
mission_close: { criteria }
```

No `waves:` key. Verified by reading `render_skeleton` end-to-end and by grepping `crates/` for `waves:`:

```bash
$ grep -n "waves:" crates/codex1/src
# (no matches)
```

The string `wave_id` does appear (`crates/codex1/src/cli/plan/waves.rs:115`, `crates/codex1/src/cli/status/next_action.rs:63`, `crates/codex1/src/cli/task/next.rs:55`) — but these are the JSON output keys for the derived projection (`codex1 plan waves`, `codex1 status`, `codex1 task next`), not stored YAML. Each call recomputes waves from `tasks[].depends_on` + current `state.tasks[id].status`. No wave list is persisted.

`crates/codex1/src/state/schema.rs` confirms the persisted shape: `MissionState` (lines 171-188) has `tasks`, `reviews`, `replan`, `close`, etc., but no `waves` field. `docs/cli-contract-schemas.md:104-119` pins the same shape.

### CC-4: Reviewer writeback positively forbidden in skill + references

`$review-loop` makes the prohibition explicit in four places:

- `.codex/skills/review-loop/SKILL.md:11`:
  > The main thread is the sole writer of mission truth: it records clean/dirty via `codex1 review record` (planned review) or `codex1 close record-review` (mission close).
- `.codex/skills/review-loop/SKILL.md:63`:
  > Every spawn prompt must begin with the standing reviewer block in `references/reviewer-profiles.md` (findings-only, no edits, no `codex1` mutations, no repairs, no marking clean anywhere). The main thread is the sole writer of mission truth. Reviewer writeback is forbidden.
- `.codex/skills/review-loop/SKILL.md:79`:
  > - Record review results from inside a reviewer subagent — only the main thread records.
- `.codex/skills/review-loop/references/reviewer-profiles.md:8-15` (standing reviewer block):
  > Do not edit files. / Do not invoke Codex1 skills. / Do not run codex1 mutating commands (no `review record`, `task finish`, `close *`, etc.). / Do not record mission truth.

The orchestration side mirrors this. `.codex/skills/execute/SKILL.md:117`:

> - Do not spawn reviewers, run `codex1 review record`, or write to `reviews/`.

Both positive ("main thread records") and negative ("reviewers must not record") forms are present.

### CC-5: No wrapper runtime / daemon / `Command::new("codex"|"claude")`

Grep over the repo:

```bash
$ grep -rIn "codex1-runtime\|ralph-daemon\|wrapper_runtime" --exclude-dir=target .
# (no matches)

$ grep -rIn 'Command::new("codex"\|Command::new("claude"' crates/codex1/src
# (no matches)
```

Every `Command::new` call across the codebase is in test fixtures invoking `bash`:

- `crates/codex1/tests/e2e_ralph_contract.rs:62, 92` — `Command::new("bash")` to exercise the Ralph hook script.
- `crates/codex1/tests/ralph_hook.rs:60, 91` — same, for the standalone hook test.

No production source path spawns any background worker, daemon, or sub-CLI. Grep for `std::process::Command|process::Command|tokio::spawn|spawn\(` across `crates/codex1/src` returns zero matches. The binary is a one-shot CLI on every invocation.

The only PATH probe is `crates/codex1/src/cli/doctor.rs:63-72` (`which_codex1`), which iterates `$PATH` looking for the binary as a self-health check. It does not invoke anything.

### CC-6: Ralph hook is status-only

`scripts/ralph-stop-hook.sh` (60 lines, owned by Phase B Unit 12):

- Sets `set -euo pipefail`.
- Drains stdin (`cat > /dev/null`) so the pipe doesn't stall (line 14).
- Resolves `$CODEX1_BIN` (default `codex1`) and verifies it is on PATH (lines 16-21).
- Runs **exactly one CLI command** (line 25):

  ```bash
  status_json="$("$CODEX1" status --json 2>/dev/null || true)"
  ```
- Parses `.data.stop.allow` with `jq` (lines 36-39) or a degraded grep fallback (lines 40-45).
- Exits 2 iff `allow == false`; exits 0 in every other case (lines 47-60).

It never reads `PLAN.yaml`, `STATE.json`, `EVENTS.jsonl`, `reviews/`, `specs/`, or `.ralph/`. It never spawns another process. The promise is reiterated in `scripts/README-hook.md:17-22`:

> The hook does **not** read plan files, subagent state, `.ralph/` directories, or anything else. It does **not** run `codex1` more than once per Stop event. It does **not** spawn helpers.

`codex1 hook snippet` (`crates/codex1/src/cli/hook.rs:24-50`) only prints the wiring JSON; it does not install, probe, or invoke anything beyond stdout.

### CC-7: No stored waves as editable truth

- `crates/codex1/src/state/schema.rs:171-188` defines `MissionState`. No `waves` field.
- `docs/cli-contract-schemas.md:104-119` pins the same shape — no `waves` collection in the canonical Rust types listing.
- `crates/codex1/src/cli/plan/waves.rs::ParsedTask` and the wave derivation in `compute_waves` (lines ~180-200) re-derive on each call from `tasks[].depends_on` + current task status. No state field is read for wave membership.
- `.codex/skills/plan/SKILL.md:199`: "Do not store waves inside `PLAN.yaml`. Waves are derived." `plan/references/dag-quality.md:15`: "Waves are derived by `codex1 plan waves` from `depends_on`."
- Grep over `.codex/skills/` for `waves:` returns zero matches. No skill body or reference emits a `waves:` YAML key.

### CC-8: No capability tokens / authority tokens / session-id authority

Grep over `crates/codex1/` for `session_id|session_token|capability_token|authority_token`: zero matches (subset of CC-2). No handler accepts, mints, stores, or validates a capability/authority token. The only stale-writer protection is `--expect-revision <N>` (`crates/codex1/src/core/error.rs:53-54` for the `RevisionConflict` variant; `crates/codex1/src/cli/mod.rs:64-65` for the global flag), which compares an integer against `state.revision`. This is artifact-validity protection, not caller-identity enforcement — exactly what `02-cli-contract.md:121` requires ("Role behavior is prompt-governed. Artifact shape and state transitions are CLI-governed.").

### CC-9: CLI does not spawn subagents

- `crates/codex1/src/cli/**/*.rs` contains no `Command::new("codex")` / `Command::new("claude")` / OpenAI / Anthropic SDK invocation. No HTTP client. No model call.
- The only stdin read in any handler is `crates/codex1/src/cli/plan/choose_level.rs:110-128` (`prompt_for_level`), and only when `stdin.is_terminal()` — i.e. for the explicitly sanctioned interactive level prompt (`02-cli-contract.md:46`).
- `codex1 task packet` and `codex1 review packet` emit prompt strings the main thread can paste into a worker/reviewer subagent prompt; neither spawns the subagent itself.

### CC-10: CLI does not ask semantic clarification questions

The only interactive prompt in the binary is `prompt_for_level` (CC-9 above), which asks one closed question (`light` / `medium` / `hard`). It does not ask for mission scope, success criteria, architecture choice, or any open-ended semantic input. Per `02-cli-contract.md:46`, this is the explicit exception to the "no surprise interactive prompts" rule.

All other commands are non-interactive; they read flags + state files and emit JSON.

### CC-11: Visible mission files only — no side-cache, no hidden dotfile

`crates/codex1/src/core/paths.rs::MissionPaths` resolves only:

- `PLANS/<mission>/OUTCOME.md`
- `PLANS/<mission>/PLAN.yaml`
- `PLANS/<mission>/STATE.json` (+ `.lock` for fs2)
- `PLANS/<mission>/EVENTS.jsonl`
- `PLANS/<mission>/specs/`
- `PLANS/<mission>/reviews/`
- `PLANS/<mission>/CLOSEOUT.md` (written by `close complete`)

No handler writes outside `PLANS/<mission>/`. There is no side-cache, side-state, or hidden dotfile. The only state-side file Foundation owns outside this set is the doctor probe at `~/.local/bin/.codex1-doctor-probe` (`crates/codex1/src/cli/doctor.rs:75-87`), which is created and immediately removed during `doctor` to test write permission. It never persists.

### CC-12: `cli-contract-schemas.md` remains foundation-owned and untouched in this audit

Per `docs/cli-contract-schemas.md:369-385`, schemas + Rust core/state files are foundation-owned. This audit added only `docs/audits/iter2-*.md` files; it did not modify any Rust source, skill body, schema, script, or non-audit doc. `cargo build --release` on the audited tree produces no diff.

## Reading map

Each anti-goal and where it was verified clean:

| Anti-goal (handoff source) | Verified at |
| --- | --- |
| `00-why-and-lessons.md:80` — "Hidden daemons." | CC-5 |
| `00-why-and-lessons.md:81` — "Wrapper runtimes around Codex." | CC-5 |
| `00-why-and-lessons.md:82` — "`.ralph` as mission truth." | CC-1 |
| `00-why-and-lessons.md:83` — "Fake permission enforcement for subagent roles." | CC-2 |
| `00-why-and-lessons.md:84` — "Caller identity checks." | CC-2 |
| `00-why-and-lessons.md:85` — "Capability token mazes." | CC-8 |
| `00-why-and-lessons.md:86` — "Reviewer writeback authority systems." | CC-4 |
| `00-why-and-lessons.md:87` — "Stored waves as editable truth." | CC-3, CC-7 |
| `00-why-and-lessons.md:88` — "Many competing closeout/gate/cache files." | CC-11 |
| `00-why-and-lessons.md:90` — "Autopilot as a separate hidden runtime." | CC-5 |
| `README.md:78` — "CLI must not detect parent vs subagent." | CC-2 |
| `README.md:79` — "No `.ralph` mission state directory." | CC-1 |
| `README.md:86` — "A wrapper runtime around Codex." | CC-5 |
| `README.md:87` — "A giant state machine hidden in hooks." | CC-6 |
| `README.md:89` — "Caller identity checks." | CC-2 |
| `README.md:90` — "Capability-token maze." | CC-8 |
| `README.md:91` — "Session-ID authority system." | CC-8 |
| `README.md:92` — "Reviewer writeback authority tokens." | CC-4, CC-8 |
| `README.md:93` — "Stored waves as canonical truth." | CC-3, CC-7 |
| `README.md:94` — "Multiple competing closeout/gate/cache truth surfaces." | CC-11 |
| `README.md:95` — "A CLI that spawns subagents." | CC-9 |
| `README.md:96` — "A CLI that asks semantic clarification questions." | CC-10 |
| `02-cli-contract.md:112` — "Ask 'are you the parent or subagent?'" | CC-2 |
| `02-cli-contract.md:113` — "Detect whether the caller is reviewer/worker/...". | CC-2 |
| `02-cli-contract.md:114` — "Block reviewer commands based on identity." | CC-2 |
| `02-cli-contract.md:115` — "Spawn subagents." | CC-9 |
| `02-cli-contract.md:116` — "Read hidden chat state." | CC-9 (no stdin parse) |
| `02-cli-contract.md:117` — "Use `.ralph` mission truth." | CC-1 |
| `02-cli-contract.md:118` — "Store waves as editable truth." | CC-3, CC-7 |
| `02-cli-contract.md:119` — "Become a giant workflow daemon." | CC-5, CC-6 |
| `01-product-flow.md:241` — "Ralph must not inspect plan/review files directly." | CC-6 |

Every anti-goal verified clean.
