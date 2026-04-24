# 09 Implementation Errata

This file captures exact implementation details that should not be left to
interpretation. It is intentionally practical: build order, hook snippets,
doctor checks, and shared invariants.

## Verified Codex Facts

These facts were checked against the local official Codex repo at
`/Users/joel/.codex/.codex-official-repo`.

- Stop-hook input includes `stop_hook_active` and related request fields in
  `codex-rs/hooks/src/events/stop.rs`.
- Stop-hook output accepts `{"decision":"block","reason":"..."}` and rejects a
  block without a non-empty reason in `codex-rs/hooks/src/events/stop.rs`.
- Inline Codex hooks are loaded from the `hooks` config field in
  `codex-rs/config/src/config_toml.rs`.
- Hook TOML uses event matcher groups and nested `.hooks` command handlers, as
  tested in `codex-rs/config/src/hooks_tests.rs`.
- Managed requirements can carry hooks under `[hooks]` with
  `[[hooks.<Event>]]` groups, as tested in
  `codex-rs/config/src/config_requirements.rs`.
- Runtime hook discovery/assembly is separate from config-shape parsing:
  `codex-rs/hooks/src/engine/discovery.rs` loads managed requirement handlers,
  hooks files, and TOML hook config layers into the active handler set.
- `codex_hooks` is a stable feature flag and currently defaults true in
  `codex-rs/features/src/lib.rs`.
- Thread fork/start APIs accept config overrides, and threads carry
  `agentRole`, in the app server protocol TypeScript schema.
- The local model catalog includes `gpt-5.5` and `gpt-5.4-mini`.

## Exact Ralph Hook Snippets

Inline `config.toml`:

```toml
[[hooks.Stop]]

[[hooks.Stop.hooks]]
type = "command"
command = "codex1 ralph stop-hook"
timeout = 5
statusMessage = "checking Codex1 mission status"
```

Managed `requirements.toml`:

```toml
[hooks]
managed_dir = "/absolute/path/to/managed/hooks"

[[hooks.Stop]]

[[hooks.Stop.hooks]]
type = "command"
command = "codex1 ralph stop-hook"
timeout = 5
statusMessage = "checking Codex1 mission status"
```

These snippets are part of the product contract. The implementation must include
tests that parse these exact snippets through the current Codex config parser.
The field names shown here are TOML config names; do not reuse them blindly for
app-server/protocol JSON surfaces, which may expose different casing such as
`timeoutSec`.

Required parser assertions:

- Inline config has exactly one `Stop` matcher group.
- Inline config has exactly one command handler.
- Managed requirements has the configured `managed_dir`.
- Managed requirements has exactly one `Stop` matcher group.
- Managed requirements has exactly one command handler.
- The command is exactly `codex1 ralph stop-hook`.

## Subagent Hook Disable Proof

Every Codex1 custom-role subagent config must include:

```toml
[features]
codex_hooks = false
```

This is not enough by itself. Add an e2e proof:

1. Install a test Stop hook that writes to a temporary marker file.
2. Spawn/start a worker role from an explicit task packet with the effective
   custom-role config.
3. Let that worker reach a normal stop.
4. Assert the marker file was not written.
5. Run the same marker hook in the main/root orchestrator context and assert it
   does run.

Do not implement this by teaching the CLI to detect parent vs subagent. The
proof is about effective Codex config, not caller identity.

## Model Policy Proof

Codex1 uses only:

```text
gpt-5.5
gpt-5.4-mini
```

`gpt-5.5` is real, available in the target Codex environment, and is the latest
best model for serious Codex1 orchestration, implementation, review, and
mission-close work. Do not hedge this policy in prompts.

`codex1 doctor --json` may report whether those models are present in the
deployment environment. It must not pick fallback models, rewrite role configs,
or influence runtime routing. If the model check fails, `doctor` fails; normal
mission commands do not contain fallback branches.

## First Vertical Slice

Build this before the full graph/review surface:

1. `codex1 --help`, `codex1 init`, and `codex1 doctor --json`.
2. `OUTCOME.md` check/ratify with schema/version rules.
3. One durable normal-mode mission path: plan scaffold/check/lock, one step,
   proof, and task finish.
4. `STATE.json` atomic revision updates and append-only `EVENTS.jsonl`.
5. `codex1 status --json` with exact `verdict`, `next_action`, `loop`,
   `close`, and `stop` projection.
6. `codex1 loop activate/pause/resume/deactivate` and `$interrupt` mapping.
7. `codex1 ralph stop-hook` with fail-open behavior and the one-block
   `stop_hook_active` rule.
8. Minimal close path for a normal mission: close check, close complete, and
   `CLOSEOUT.md`.
9. First-slice skill wrappers for `$clarify`, `$plan`, `$execute`,
   `$interrupt`, and minimal `$autopilot`, proving users experience Codex1
   through skills rather than raw CLI commands. The exact first-slice skill
   wrapper contract is in `10-first-slice-skill-contracts.md`.

Do not implement graph waves, planned review tasks, repair budgets, or
mission-close review until this slice works end to end from outside the source
folder.

## Active Mission Pointer

Use `PLANS/ACTIVE.json` as visible repo-local selection metadata for Ralph and
status defaults:

```json
{
  "schema_version": "codex1.active.v1",
  "mission_id": "codex1-rebuild",
  "selected_at": "2026-04-24T10:00:00Z",
  "selected_by": "codex1 loop activate",
  "purpose": "ralph_status_default"
}
```

Rules:

- `ACTIVE.json` is metadata, not mission truth.
- Mission state remains in `PLANS/<mission-id>/STATE.json`.
- If `cwd` is inside `PLANS/<mission-id>/`, that mission wins over the pointer.
- `codex1 loop activate --mission <id>` writes or updates the pointer.
- `$interrupt` / `codex1 loop pause` should keep the pointer and make Ralph
  allow stop through paused status.
- `codex1 loop deactivate` and `codex1 close complete` should clear the pointer
  if it points at that mission.
- Missing, invalid, stale, or mismatched pointer data makes Ralph allow stop.

Do not use `PLANS/.active`; keep the pointer visible and schema-versioned.

## Doctor Drift Check

`doctor` should compare `STATE.json.revision` with the latest appended event
revision in `EVENTS.jsonl`.

If state is ahead of events, report audit drift:

```json
{
  "id": "state_event_revision_drift",
  "ok": false,
  "state_revision": 18,
  "latest_event_revision": 17,
  "severity": "warning",
  "message": "STATE.json is authoritative, but EVENTS.jsonl is missing audit for revision 18."
}
```

This is diagnostic only. `doctor` must not rewrite mission state or invent audit
events.

## Status Is The Product Artifact

`codex1 status --json` should be treated as the central integration artifact.

If status is exact:

- Ralph is simple.
- Skills can stay thin.
- Humans can debug state.
- Tests can assert behavior without reading every artifact.

If status is fuzzy, the rest of the system turns into ceremony. Prefer adding
tests to status projection over adding more orchestration logic.

## Shared Code Invariants

These invariants must live in shared code, not be duplicated across commands:

- State load, schema validation, revision check, atomic write, and event append.
- Status projection.
- Stop projection.
- Close readiness.
- Graph wave derivation.
- Review freshness and stale-review detection.
- Review repair budget accounting.
- Plan digest calculation.

The commands may format different JSON outputs, but they should not recalculate
these truths independently.

## Appetite Guard

The first implementation should feel slightly boring:

- One status engine.
- One close-readiness engine.
- One Ralph adapter.

Add the graph wave derivation and review freshness functions only when the graph
and review slice begins.

If implementation starts by creating a large module tree for every future
command, pause and return to the first vertical slice.
