# Internal Command Migration Plan

This document converts the internal command taxonomy proposal into a staged,
repo-owned migration plan.

It is intentionally incremental. The goal is to move toward clearer canonical
command names without breaking existing repo callers, qualification flows, or
external skill wiring in one large rename.

## Objectives

- Make the deterministic `codex1 internal ...` surface easier for Codex to use
  directly through stable, family-oriented command names.
- Keep legacy command names working during migration.
- Update docs first, then migrate repo-owned callers, then tighten the surface
  with narrower helpers where it materially improves reliability.

## Phase 1: Canonical Names And Compatibility Aliases

Status: active in the repo.

Scope:

- Introduce canonical command names in the CLI help surface.
- Preserve existing names as compatibility aliases.
- Update runtime docs to prefer canonical names and call out the aliases.
- Keep repo-owned callers on legacy names unless there is a specific reason to
  switch them immediately.

Canonical names introduced in Phase 1:

- `materialize-plan`
- `record-review-outcome`
- `append-replan-log`
- `append-closeout`
- `repair-state`
- `validate-mission-artifacts`
- `inspect-effective-config`
- `clear-selection-wait`

Legacy aliases preserved in Phase 1:

- `write-blueprint`
- `record-review-result`
- `write-replan-log`
- `write-closeout`
- `rebuild-state`
- `validate-artifacts`
- `effective-config`
- `consume-selection`

Exit criteria:

- `codex1 internal --help` shows the canonical names.
- Legacy names still dispatch successfully.
- Runtime docs point new work toward canonical names.

## Phase 2: Migrate Repo-Owned Callers

Status: active.

Scope:

- Update repo-owned Rust callers, qualification flows, tests, and fixture docs
  to use the canonical names.
- Preserve the legacy aliases for compatibility with older skills, pinned
  threads, and external docs while the repo migrates.

Targets in this phase:

- `crates/codex1/src/commands/qualify.rs`
- `crates/codex1/tests/runtime_internal.rs`
- any helper scripts or fixture docs that shell out to `codex1 internal ...`
- repo-local public skill docs under `.codex/skills/`

Rules:

- Migrate in small batches to keep failures easy to localize.
- Prefer updating high-signal call sites first:
  - qualification flows
  - runtime integration tests
  - examples in docs
- Do not remove aliases in this phase.

Completed in the current wave:

- `crates/codex1/src/commands/qualify.rs` now prefers canonical names.
- `crates/codex1/tests/runtime_internal.rs` now prefers canonical names.
- repo-local skill docs for `plan`, `execute`, and `review` now prefer
  canonical names.

Exit criteria:

- Repo-owned callers prefer canonical names by default.
- Qualification and runtime tests pass on canonical names.
- No required repo path depends exclusively on legacy names.

## Phase 3: Narrow Overloaded Commands

Status: active groundwork.

Scope:

- Split broad wrapper commands only where that materially improves
  inspectability or skill control.

Primary candidates:

- `materialize-plan`
  - possible later helpers:
    - `write-blueprint`
    - `sync-workstream-specs`
    - `sync-execution-graph`
    - `sync-planning-gates`
- `record-review-outcome`
  - possible later helpers:
    - `write-review-ledger`
    - `update-review-gates`
    - `append-closeout`
- `validate-mission-artifacts`
  - possible later helpers:
    - `validate-visible-artifacts`
    - `validate-machine-state`
    - `validate-execution-graph`
    - `validate-gates`
    - `validate-closeouts`
- `stop-hook`
  - keep as the only hook-facing adapter, but reduce internal branching by
    extracting a deterministic `resolve-stop-decision` helper

Completed groundwork in the current wave:

- added `validate-gates`
- added `validate-closeouts`
- added `latest-valid-closeout`
- moved the real stop decision path into `codex1-core` so `codex1 internal stop-hook`
  can stay a thin hook adapter instead of carrying duplicated mission-state
  branching in the CLI crate

Rule:

- Do not split commands purely for taxonomy aesthetics.
- Split only when a skill or repair flow genuinely benefits from a narrower,
  reusable deterministic boundary.

Exit criteria:

- The remaining wrappers are honest wrappers with understandable scope.
- New helper commands remove real complexity rather than creating ceremony.

## Phase 4: Deprecation And Alias Removal

Status: deferred.

This phase should only happen after:

- canonical names are used by repo-owned callers
- qualification covers the canonical surface
- skills and docs have been updated long enough to make removal low-risk

Possible actions:

- mark legacy aliases as deprecated in docs
- remove legacy aliases in a major compatibility window

This phase is intentionally not active now.

## Non-Goals

This migration must not:

- turn `codex1 internal` into a hidden orchestration runtime
- move mission judgment from skills into deterministic commands
- turn one broad command rename into a breaking repo-wide churn event
- remove legacy names before canonical names are proven in real flows

## Decision Rule

When deciding whether a behavior belongs in a CLI command during migration:

- If it is a deterministic transform, validator, append-only writer, inspector,
  or repair helper, prefer a CLI command.
- If it decides mission meaning, route choice, review judgment, or honest
  completion, keep it in the skill/orchestration layer.
