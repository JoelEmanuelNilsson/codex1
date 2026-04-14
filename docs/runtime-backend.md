# Codex1 Runtime Backend

This document describes the deterministic internal command surface that backs the
public Codex1 skills.

The product surface is still the skills:

- `$clarify`
- `$plan`
- `$execute`
- `$review`
- `$autopilot`

The commands below are the stable machine-side helpers those skills can call to
persist mission truth under `PLANS/` and `.ralph/`.

## Command surface

All commands live under `codex1 internal ...` and support `--json`.

`codex1 internal stop-hook` remains the single hook-facing adapter, but the
artifact-driven stop decision now lives in `codex1-core` so the CLI crate does
not carry duplicate Ralph branching logic.

## Migration status

Phase 1 of the internal command taxonomy migration is active.

- Canonical command names now follow the proposed family-oriented taxonomy.
- Legacy command names remain supported as compatibility aliases.
- Repo-owned callers may continue using legacy names during the migration, but
  new docs and new skill wiring should prefer the canonical names below.

Canonical -> legacy alias mappings introduced in Phase 1:

- `materialize-plan` -> `write-blueprint`
- `record-review-outcome` -> `record-review-result`
- `append-replan-log` -> `write-replan-log`
- `append-closeout` -> `write-closeout`
- `repair-state` -> `rebuild-state`
- `validate-mission-artifacts` -> `validate-artifacts`
- `inspect-effective-config` -> `effective-config`
- `clear-selection-wait` -> `consume-selection`

### Mission bootstrap

- `codex1 internal init-mission`
  - input: `MissionInitInput`
  - creates the visible mission root and hidden Ralph root
  - writes `README.md`, `MISSION-STATE.md`, `OUTCOME-LOCK.md`, `REVIEW-LEDGER.md`
  - creates initial `gates.json`
  - appends an initial clarify closeout

### Planning writeback

- `codex1 internal materialize-plan`
  - legacy alias: `codex1 internal write-blueprint`
  - input: `PlanningWriteInput`
  - writes `PROGRAM-BLUEPRINT.md`
  - serializes machine planning truth for `risk_floor`, `proof_matrix`, and `decision_obligations` into blueprint frontmatter instead of leaving those contracts as prose-only notes
  - rejects approved blueprint bodies that are missing the canonical visible sections required for route truth
  - rejects approved planning when the selected rigor falls below the computed risk floor or when blocking decision obligations remain unresolved
  - persists the selected execution target in blueprint frontmatter when one is known
  - writes or refreshes frontier `SPEC.md` files and support files, and rejects spec bodies that omit the canonical visible workstream sections
  - writes `.ralph/missions/<mission-id>/execution-graph.json` when the runnable frontier has non-trivial sequencing or a wave target, and removes stale graph state when planning collapses back to a trivial single-spec runnable frontier
  - opens or refreshes planning gate state, but does not pass planning completion by itself
  - appends a planning closeout
  - the runtime now stages this internally as planning preparation, spec sync,
    execution-graph sync, runtime gate/readme refresh, and closeout synthesis
    while keeping the command surface stable

### Execution package flow

- `codex1 internal compile-execution-package`
  - input: `ExecutionPackageInput`
  - compiles one hidden execution package under `.ralph/missions/<mission-id>/execution-packages/`
  - requires the package target to match the blueprint-selected target
  - requires included specs to be active runnable frontier specs
  - binds package freshness, dependency truth, and wave frontier safety to the current execution-graph contract when one is required
  - fails the package when declared dependencies are unsatisfied or when the target drifts from the included spec set
  - evaluates the package gate
  - passes planning completion only when the selected target packages honestly
  - updates `gates.json`
  - appends an execution-package closeout

- `codex1 internal validate-execution-package --mission-id <id> --package-id <id>`
  - checks current freshness against the governing lock, blueprint, execution graph, included specs, target binding, dependency truth, and package gate history

- `codex1 internal derive-writer-packet`
  - input: `WriterPacketInput`
  - derives one bounded write brief from a passed execution package
  - rejects packet targets that are not authorized by the source package
  - writes the packet under `.ralph/missions/<mission-id>/packets/`

### Review flow

- `codex1 internal compile-review-bundle`
  - input: `ReviewBundleInput`
  - writes one review bundle under `.ralph/missions/<mission-id>/bundles/`
  - rejects spec-review bundles that drift outside the source package's authorized spec set
  - folds visible review design from `PROGRAM-BLUEPRINT.md` and visible review/proof expectations from `SPEC.md` into the generated bundle
  - opens the matching review gate in `gates.json`

- `codex1 internal validate-review-bundle --mission-id <id> --bundle-id <id>`
  - checks the governing package, bundle completeness, mission-close visible truth, and review-gate freshness

- `codex1 internal record-review-outcome`
  - legacy alias: `codex1 internal record-review-result`
  - input: `ReviewResultInput`
  - updates `REVIEW-LEDGER.md`
  - updates per-spec `REVIEW.md` when the bundle is spec-local
  - updates the matching review gate
  - appends a review closeout
  - the runtime now stages this internally as review validation, review-gate
    update, visible artifact writeback, and closeout synthesis while keeping the
    command surface stable
  - requires `waiting_request` when review disposition branches to `needs_user`

### Replan and contradiction flow

- `codex1 internal record-contradiction`
  - input: `ContradictionInput`
  - appends a structured contradiction to `.ralph/missions/<mission-id>/contradictions.ndjson`

- `codex1 internal append-replan-log`
  - legacy alias: `codex1 internal write-replan-log`
  - input: `ReplanLogInput`
  - appends a visible entry to `PLANS/<mission-id>/REPLAN-LOG.md`
  - records preserved work, invalidated work, and evidence refs for non-local replans

- `codex1 internal append-closeout --mission-id <id>`
  - legacy alias: `codex1 internal write-closeout --mission-id <id>`
  - input: `CloseoutRecord`
  - appends one explicit non-terminal Ralph closeout and refreshes `state.json`
  - rejects terminal verdicts such as `complete` and `hard_blocked`; those must come from reviewed workflow-specific paths

### Resume-selection flow

- `codex1 internal resolve-resume`
  - inspects an explicit `--mission-id`, any existing `.ralph/selection-state.json`, and the current non-terminal mission set
  - returns either a ready mission binding, a durable selection wait, or a no-mission result
  - preserves an existing open selection request across repeated resumes instead of inventing a new request id
  - repairs impossible repo-level selection waits instead of preserving them forever
  - when called with an explicit `--mission-id` while a selection wait is open, supersedes the stale repo-level chooser before returning the selected mission binding

- `codex1 internal open-selection-wait`
  - input: `SelectionStateInput`
  - requires at least two distinct candidate missions
  - writes `.ralph/selection-state.json`

- `codex1 internal resolve-selection-wait`
  - input: `SelectionResolutionInput`
  - marks a chooser as resolved without clearing it yet so resume arbitration can consume it explicitly

- `codex1 internal acknowledge-selection-request`
  - input: `SelectionAcknowledgementInput`
  - marks the current selection request as durably emitted without changing the selection choice

- `codex1 internal clear-selection-wait --mission-id <id>`
  - legacy alias: `codex1 internal consume-selection --mission-id <id>`
  - clears a previously resolved selection state after the chosen mission has been bound for resume
  - validates the selected mission before clearing so stale prompts are not silently consumed

- `codex1 internal acknowledge-waiting-request --mission-id <id>`
  - input: `WaitingRequestAcknowledgementInput`
  - appends a Ralph closeout that marks the current mission waiting request as durably emitted

### Inspection and repair

- `codex1 internal inspect-effective-config`
  - legacy alias: `codex1 internal effective-config`
  - emits the current effective supported-environment view for the target repo

- `codex1 internal repair-state --mission-id <id>`
  - legacy alias: `codex1 internal rebuild-state --mission-id <id>`
  - rebuilds cached Ralph mission state from higher-authority files when the
    cached machine state has drifted or gone stale

- `codex1 internal validate-visible-artifacts --mission-id <id>`
  - validates visible mission truth under `PLANS/<mission-id>/`
  - checks parseability of `MISSION-STATE.md`, `OUTCOME-LOCK.md`,
    `PROGRAM-BLUEPRINT.md`, and visible `specs/*/SPEC.md`

- `codex1 internal validate-machine-state --mission-id <id>`
  - validates hidden Ralph mission truth under `.ralph/missions/<mission-id>/`
  - checks execution-graph validity, gate and closeout structure, active-cycle
    parseability, and whether cached `state.json` matches authoritative rebuild
    from hidden mission files

- `codex1 internal validate-gates --mission-id <id>`
  - validates and summarizes `gates.json` for one mission

- `codex1 internal validate-closeouts --mission-id <id>`
  - validates and summarizes `closeouts.ndjson` for one mission

- `codex1 internal latest-valid-closeout --mission-id <id>`
  - resolves the latest schema-valid closeout after applying the same
    truncated-tail tolerance as the Ralph loader

## Execution graph

- `execution-graph.json` is now the persisted machine graph for non-trivial sequencing.
- Graph authoring and validation bind only to runnable frontier specs; active descoped or non-runnable specs do not force graph nodes.
- Graph validation binds mission id, blueprint revision and fingerprint, active spec coverage, dependency topology, per-node scope declarations, and obligation coverage for each declared acceptance check.
- `codex1 internal validate-mission-artifacts --mission-id <id>` remains the umbrella validator and now includes explicit `visible_artifacts` and `machine_state` subreports alongside the combined findings list.
  - legacy alias: `codex1 internal validate-artifacts --mission-id <id>`

## Usage notes

- Pass JSON on stdin with `--input-json -` or point `--input-json` at a file.
- The skill remains responsible for the reasoning and the prose.
- The internal command remains responsible for durable state, fingerprints,
  gates, and closeouts.
- Ralph closeout commits now hold one commit boundary across append, state
  rewrite, and active-cycle cleanup so same-mission races cannot silently reuse
  stale closeout history.
- If governing truth changes, recompile the package or bundle instead of
  silently reusing stale machine state.
- `gates.json` is now append-preserving history rather than a last-write-wins
  snapshot; newer packages and bundles supersede or stale older gate records.
