# Qualification Contract

`codex1 qualify-codex` records an inspectable report for the exact target repo and the exact Codex CLI build used for the run.

## Current scope

- Captures the live `codex --version` result when `--live` is enabled.
- When `--live` is enabled, proves a native `codex exec` session can be resumed through `codex exec resume` on the exact trusted build under test.
- When `--live` is enabled, proves the native child-agent surface can drive the resume-critical inspection path (`spawn_agent`, `list_agents`, `wait_agent`, and `close_agent`), then feeds the live child snapshot back into Codex1 resume reconciliation. Queue-only and turn-triggering child-delivery tools are still recorded observationally when the trusted build surfaces them.
- Verifies the target repo has project-scoped `.codex/config.toml` and `.codex/hooks.json`.
- Verifies the target repo is trusted by Codex and that the effective trusted config resolves the required baseline keys honestly.
- Verifies the target repo has the Codex1-managed `AGENTS.md` scaffold block.
- Smoke-checks that `.codex/config.toml` enables `features.codex_hooks = true`.
- Smoke-checks that `.codex/hooks.json` exposes exactly one authoritative Stop-hook pipeline while tolerating observational Stop hooks.
- Verifies the combined user/project Stop-hook surface still resolves to one authoritative Ralph pipeline.
- Verifies the target repo has a discoverable skill surface through `copied_skills`, `linked_skills`, or `skills_config_bridge`.
- Runs an isolated temp-repo flow through `setup`, `doctor`, `restore`, and `uninstall`, then checks the sandbox repo and temporary user `.codex` baseline were restored.
- Runs helper-flow smoke commands under an isolated sandbox `HOME` / `CODEX_HOME` so outer user config does not leak into qualification.
- Proves helper repair/fail-safe behavior for multi-Stop conflict rejection plus `--force` normalization, repair from a deliberately partial support surface representative of interrupted setup, and drift detection on managed shared files.
- Runs an isolated temp-repo mission-runtime flow through mission bootstrap, canonical blueprint/spec writeback, execution-package compilation, writer-packet derivation, review-bundle compilation, contradiction logging, resume-resolution, and selection consumption.
- Proves that a durable `needs_user` mission and a resolver-created resume-selection wait both yield through the Stop hook, and that the acknowledgement handshake preserves request identity across idempotent re-emission.
- Proves the scoped Ralph control-loop boundary with an installed Stop hook:
  parent turns without an active lease yield, subagent turns yield, active
  parent loop leases block on owed review gates, and paused leases yield again.
- When `--live` is enabled, proves the trusted build dispatches the real native Stop hook through Codex rather than only the internal adapter.
- Proves internal backend parity by running the same mission truth through an explicit manual backend sequence and an autopilot-style backend composition, then comparing their validated durable artifact summaries.
- Proves the `$review-loop` decision contract for clean continuation,
  non-clean repair before the cap, and six consecutive non-clean loops routing
  to replan.
- Proves the reviewer-lane capability boundary by rejecting contaminated
  child-review writeback, validating a frozen review evidence snapshot, and
  accepting clean parent-owned snapshot-backed review writeback.
- Proves the delegated-review authority boundary by checking public docs forbid
  parent self-review, rejecting review outcomes without reviewer-agent output
  evidence, and rejecting durable writeback without a review truth snapshot.
- Verifies a real self-hosting source-repo gate when the target repo looks like the `codex1` source workspace.

## Live native proof boundary

`qualify-codex` now proves the machine-readable native Codex path through `codex exec` and `codex exec resume`.

The plain interactive `codex resume` TUI surface is still terminal-shaped and not part of the automated qualification flow yet. The report should therefore be read as:

- strong proof for support surfaces, helper repair/fail-safe behavior, runtime state, internal backend parity, waiting behavior, `codex exec resume`, and native child-agent tool usage on the current build
- not yet a full pseudo-terminal automation proof of the interactive TUI resume experience

For the current diagnosis of the remaining native child-agent qualification gap,
see [native-multi-agent-resume-note.md](/Users/joel/codex1/docs/qualification/native-multi-agent-resume-note.md).

## Autonomy Governance Proof Boundary

The fully autonomous execute or autopilot promise is not proven by one gate.
It is the combination of:

- package-entry honesty for the selected target
- manual or autopilot parity over the same durable mission truth
- durable waiting identity through stop-hook and resume flows
- no-false-terminal behavior during review, repair, replan, and child-lane
  reconciliation
- raw-evidence judgment for native child-lane inspection and resume

Those proof surfaces together are the evidence boundary for Codex1's autonomy
claim on the supported native Codex build.

The public skill-level branch contract is proven separately from those
qualification gates by targeted repository tests that assert:

- `$execute` explicitly routes a clean final frontier into mission-close review
- `$autopilot` explicitly routes that same condition into `$review-loop`
- `docs/runtime-backend.md` preserves the same public-ownership contract

Qualification parity and waiting gates support that claim, but they do not
replace the direct public-contract proof.

## Support-surface baseline

The current supported helper baseline is:

- `model = "gpt-5.4"`
- `review_model = "gpt-5.4-mini"`
- `model_reasoning_effort = "high"`
- `[codex1_orchestration] model = "gpt-5.4"`
- `[codex1_orchestration] reasoning_effort = "high"`
- `[codex1_review] model = "gpt-5.4-mini"`
- `[codex1_review] reasoning_effort = "high"`
- `[codex1_fast_parallel] model = "gpt-5.3-codex-spark"`
- `[codex1_fast_parallel] reasoning_effort = "high"`
- `[codex1_hard_coding] model = "gpt-5.3-codex"`
- `[codex1_hard_coding] reasoning_effort = "xhigh"`
- `features.codex_hooks = true`
- `agents.max_threads = 16`
- `agents.max_depth = 1`

## Evidence layout

Each run writes:

- Versioned evidence to `<repo>/.codex1/qualification/reports/<timestamp>--<codex-version>--<id>.json`
- Per-gate evidence payloads to `<repo>/.codex1/qualification/reports/<timestamp>--<codex-version>--<id>/`
- The latest report to `<repo>/.codex1/qualification/latest.json`

The report schema is currently `codex1.qualify.v1`.

## JSON contract

When `--json` is set, stdout is the full qualification report. The command still exits non-zero if any required gate fails, so callers can keep JSON parsing and shell gating at the same time.

The top-level JSON keys are:

- `schema_version`
- `qualification_id`
- `qualified_at`
- `tested_at` (compatibility alias during the schema transition)
- `repo_root`
- `requested`
- `codex_build`
- `support_surface_signature`
- `summary`
- `gates`
- `evidence`

Each gate may now also include `evidence_path`, which points at the persisted
raw artifact payload used to justify that gate's pass, fail, or skip outcome.

For the live native child-lane gate specifically, the decisive judgment path is
now the raw JSONL tool/event stream plus the raw wait or resume artifacts it
produced. Any final model-authored JSON summary is retained only as convenience
evidence and must not override the raw event record.

## Source-repo invocation

The self-hosting gate is meant to be invoked from the source workspace after `codex1 setup` has materialized the managed support surfaces:

```bash
cargo run -p codex1 -- qualify-codex --repo-root /Users/joel/codex1 --json
```

That run should now pass the self-hosting gate when the source repo contains the expected `codex1` crates, PRD marker, and managed support surfaces.
