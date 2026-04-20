# Codex1

Skills-first native Codex workflow harness.

Codex1 turns long-running Codex missions (clarify → plan → execute → review → close) into visible, auditable work. You invoke one of six public skills — `$clarify`, `$plan`, `$execute`, `$review-loop`, `$close`, `$autopilot` — and they drive a small deterministic `codex1` CLI that validates, records, and projects mission state.

Ralph, the stop guard, only reads `codex1 status --json`. There is no hidden `.ralph/` state, no caller-identity enforcement, and no stored waves.

> **Build state:** this is the Foundation cut. Phase B feature commands (`outcome`, `plan`, `task`, `review`, `replan`, `loop`, `close`, full `status`) are scaffolded and return `NOT_IMPLEMENTED`. Phase B PRs implement them.

## Install

```bash
git clone https://github.com/joel-nilsson/codex1 && cd codex1
make install-local                 # builds release + copies to ~/.local/bin/codex1
export PATH="$HOME/.local/bin:$PATH"
cd /tmp && command -v codex1       # verify from outside the source folder
codex1 --help
codex1 doctor                      # never crashes; reports CLI health as JSON
```

## Minimum verification

```bash
make verify-contract    # fmt + clippy + cargo test + install-local + smoke tests
```

## Try a mission (Foundation can do this much)

```bash
cd /tmp && mkdir demo && cd demo
codex1 init --mission demo          # creates PLANS/demo/ with STATE.json, OUTCOME.md, PLAN.yaml
cat PLANS/demo/STATE.json           # revision:0, phase:"clarify"
codex1 status --mission demo        # verdict:"needs_user"; stop.allow:true
```

Further commands (`codex1 outcome …`, `codex1 plan …`, etc.) land in Phase B. The clap tree is already wired, so `codex1 --help` shows the full command surface today.

## Layout

```
Cargo.toml                 # workspace; pins dependency versions
crates/codex1/             # single binary + library
  src/bin/codex1.rs        # thin entry point
  src/lib.rs               # declares modules + run()
  src/cli/                 # clap dispatch + per-command subcommands
  src/core/                # envelope, error codes, mission resolution, config
  src/state/               # STATE.json / EVENTS.jsonl with atomic writes + fs2 locks
  tests/foundation.rs      # contract-surface integration tests
Makefile                   # install-local, verify-installed, verify-contract
docs/cli-contract-schemas.md   # authoritative JSON envelope + error code reference
docs/codex1-rebuild-handoff/   # normative product docs (6 files)
.codex/skills/             # six public skills (Phase B)
scripts/                   # Ralph stop hook (Phase B)
```

## Skills-first, CLI-on-rails

- **Skills** (`$clarify`, `$plan`, `$execute`, `$review-loop`, `$close`, `$autopilot`) are the user product.
- **CLI** (`codex1`) is the deterministic substrate that validates artifacts, records state, and emits stable JSON.
- **Visible files** under `PLANS/<mission-id>/` own mission truth. There is no hidden state.
- **Subagents** are governed by prompts, not by CLI identity checks.
- **Ralph** is a tiny Stop hook that asks `codex1 status --json` whether to allow termination.

## Contract (full spec in `docs/cli-contract-schemas.md`)

Every command:
- Supports `--help`, `--json`, `--mission <id>`, `--repo-root <path>`.
- Mutating commands also support `--dry-run` and `--expect-revision <N>` (strict equality; returns `REVISION_CONFLICT` on mismatch).
- Emits JSON on stdout. Success envelope: `{"ok":true,"mission_id":…,"revision":…,"data":…}`. Error envelope: `{"ok":false,"code":…,"message":…,"hint":…,"retryable":…,"context":…}`.
- Exits `0` on success (including empty results), `1` on handled error, `2` on harness bug.

## Licensing

MIT. See workspace `Cargo.toml` for the metadata.
