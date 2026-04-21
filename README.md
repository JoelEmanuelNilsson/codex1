# Codex1

Codex1 is a skills-first native Codex workflow. You invoke one of six public skills — `$clarify`, `$plan`, `$execute`, `$review-loop`, `$close`, `$autopilot` — and the skill drives a small deterministic `codex1` CLI on rails: the CLI validates artifacts, records mission state, derives execution waves, and emits stable JSON for Ralph, a tiny Stop hook that only reads `codex1 status --json`. All mission truth lives in visible files under `PLANS/<mission-id>/`. There is no hidden `.ralph/` state, no caller-identity enforcement, and no stored waves.

> **Build state.** The CLI surface is implemented end to end: `init`, `doctor`, `hook snippet`, `status`, `outcome`, `plan`, `task`, `review`, `replan`, `loop`, and `close` all emit stable JSON envelopes. `codex1 --help` shows the full command tree.

## Quick start

```bash
make install-local           # builds release + copies to ~/.local/bin/codex1
export PATH="$HOME/.local/bin:$PATH"
codex1 --help                # shows the full command tree
codex1 --json doctor         # health check; never crashes
codex1 --json init --mission demo   # creates PLANS/demo/
```

Verify the install from outside the source folder. See [`docs/install-verification.md`](docs/install-verification.md) for the `/tmp` recipe.

## Skills

The six public skills are the user-facing product. Each is invoked by its `$` name in a Codex session.

| Skill | Purpose |
| --- | --- |
| `$clarify` | Interview the user until `OUTCOME.md` is ratifiable, then ratify it. |
| `$plan` | Produce a full mission plan with a valid task DAG; lock it with `codex1 plan check`. |
| `$execute` | Run the next ready task or parallel-safe wave from the DAG. |
| `$review-loop` | Orchestrate reviewer subagents for a planned review task or mission-close review; record outcomes via `codex1 review record`. |
| `$close` | Pause the active loop so the user can discuss without Ralph forcing continuation. |
| `$autopilot` | Compose the whole manual flow end-to-end, pausing for genuine user input. |

The skills ship from Phase B under `.codex/skills/`. The workflow each skill drives is documented in [`docs/codex1-rebuild-handoff/01-product-flow.md`](docs/codex1-rebuild-handoff/01-product-flow.md).

## Manual CLI flow

For users (or automation) driving the mission without skills, the end-to-end CLI sequence is:

```bash
codex1 init --mission m1
# edit PLANS/m1/OUTCOME.md
codex1 outcome check --mission m1
codex1 outcome ratify --mission m1

codex1 plan choose-level --mission m1 --level medium
codex1 plan scaffold --mission m1 --level medium
# edit PLANS/m1/PLAN.yaml
codex1 plan check --mission m1
codex1 plan waves --mission m1

codex1 task next --mission m1
codex1 task start T1 --mission m1
# do the work; write PLANS/m1/specs/T1/PROOF.md
codex1 task finish T1 --proof PLANS/m1/specs/T1/PROOF.md --mission m1

# repeat for each task; planned review tasks use `review packet` + `review record`.

codex1 close check --mission m1
codex1 close complete --mission m1
```

Every mutating command accepts `--expect-revision <N>` for strict-equality stale-writer protection. Every command supports `--json` (default), `--help`, `--mission <id>`, and `--repo-root <path>`. Per-command shapes and error codes are in [`docs/cli-reference.md`](docs/cli-reference.md).

## Visible mission files

A mission is everything under `PLANS/<mission-id>/`. `OUTCOME.md` is the clarified destination; `PLAN.yaml` is the route and task DAG; `STATE.json` is the operational state (CLI-owned, never edited by hand); `EVENTS.jsonl` is the append-only audit log; `specs/T<id>/{SPEC,PROOF}.md` are task-local; `reviews/*.md` are main-thread-recorded review findings; `CLOSEOUT.md` is the terminal summary.

File-by-file ownership, mutation protocol, revision discipline, and late-output vocabulary are in [`docs/mission-anatomy.md`](docs/mission-anatomy.md).

## Ralph stop hook

Ralph is tiny. It runs `codex1 status --json` and blocks the Stop event iff the loop is active, not paused, and `stop.allow` is false. Ralph does not read plan or review files, does not manage subagents, and does not keep its own state.

Print the wiring one-liner:

```bash
codex1 --json hook snippet
```

The shell script and its install guide ship from Phase B Unit 12; see [`scripts/README-hook.md`](scripts/README-hook.md) once that unit lands.

## Contract

Every command emits a stable JSON envelope on stdout. Success shape:

```json
{ "ok": true, "mission_id": "demo", "revision": 7, "data": { /* command-specific */ } }
```

Error shape:

```json
{ "ok": false, "code": "PLAN_INVALID", "message": "...", "hint": "...", "retryable": false, "context": { /* ... */ } }
```

Exit codes: `0` success, `1` handled error, `2` harness bug (IO / JSON / YAML). The authoritative envelope, error-code set, STATE.json schema, mutation protocol, verdict derivation, and per-command data shapes are in [`docs/cli-contract-schemas.md`](docs/cli-contract-schemas.md). Phase B must not modify that file.

## Install verification

`make verify-contract` runs `fmt` + `clippy` + `test` + `install-local` + the installed-binary smoke check. The step-by-step install, PATH, and verify-from-`/tmp` recipe is in [`docs/install-verification.md`](docs/install-verification.md).

## Layout

```
Cargo.toml                           # workspace; pins dependency versions
Makefile                             # install-local, verify-installed, verify-contract
crates/codex1/                       # single binary + library
  src/bin/codex1.rs                  # thin entry point
  src/lib.rs                         # declares modules + run()
  src/cli/                           # clap dispatch + per-command subcommands
  src/core/                          # envelope, error codes, mission resolution, config
  src/state/                         # STATE.json / EVENTS.jsonl with atomic writes + fs2 locks
  tests/                             # contract-surface integration tests
docs/
  cli-contract-schemas.md            # authoritative envelope + error code reference
  cli-reference.md                   # per-command reference (this documentation set)
  install-verification.md            # install + verify-from-/tmp recipe
  mission-anatomy.md                 # PLANS/<id>/ file-by-file anatomy
  codex1-rebuild-handoff/            # normative product docs (6 files)
.codex/skills/                       # six public skills (Phase B)
scripts/                             # Ralph stop hook (Phase B)
```

## License

MIT. See the workspace `Cargo.toml` metadata block.
