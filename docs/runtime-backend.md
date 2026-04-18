# Codex1 Runtime Backend

This document describes the deterministic internal command surface that backs the
public Codex1 skills.

The product surface is still the skills:

- `$clarify`
- `$plan`
- `$execute`
- `$review-loop`
- `$autopilot`
- `$close`

## Public Skill Responsibilities

The deterministic backend exists to persist and validate mission truth. It does
not replace the public workflow contracts owned by the skills.

- `$clarify` owns ambiguity destruction, destination truth, and lock readiness.
  It should ask the highest-leverage next question, keep provenance explicit in
  `MISSION-STATE.md`, and ratify `OUTCOME-LOCK.md` only when the destination is
  genuinely planning-safe. Manual clarify stops at a durable handoff asking the
  user to invoke `$plan`; `$autopilot` may consume that handoff and continue
  into planning.
- `$plan` owns route quality, critique, packetization, execution-graph truth,
  and packaging honesty. It should not stop at document drafting; it must carry
  the route through to a passed execution package for the next selected target.
- `$execute` owns bounded target advancement under a passed package, including
  honest routing into review, mission-close review, repair, replan, and durable
  waiting branches.
- `$review-loop` owns parent/orchestrator review waves, fresh bundle-bound
  judgment, explicit findings, and mission-close honesty. It must not clear
  stale or weakly evidenced work.
- `$autopilot` owns no separate semantics; it is the branch-routing composition
  over clarify, plan, execute, and review-loop, and must preserve manual-path
  parity.
- `$close` owns the user escape hatch for active parent loop leases. It pauses
  or clears Ralph continuation state so discussion and redirection can stop
  normally without moving or uninstalling hook files.

`$review-loop` review profiles are part of the public workflow contract:

- local/spec intent judgment uses `gpt-5.4`
- integration intent judgment uses `gpt-5.4`
- mission-close judgment uses two `gpt-5.4` reviewers
- code bug/correctness review uses `gpt-5.3-codex` at proof-worthy code
  checkpoints

Child reviewer outputs are findings-only: `NONE` or structured findings with
severity and evidence refs. P0/P1/P2 findings block clean review; P3 findings
are non-blocking by default. Six consecutive non-clean loops route to replan.

The Stop-hook input may identify a child lane as `findings_only_reviewer` via
lane metadata. Those review child lanes are allowed to return their bounded
findings payload without being blocked by parent mission gates. Parent
controller lanes still apply normal Ralph blocking, and only the parent records
review outcomes, gates, ledgers, closeouts, or completion.

Parent-owned review writeback is not parent-owned review judgment. The parent
may orchestrate, aggregate, detect contamination, route repair/replan, and write
the durable outcome, but code/spec/intent/integration/mission-close review
judgment must come from reviewer-agent outputs. Accepted review outcomes must
cite reviewer-agent output evidence refs backed by bounded inbox artifacts,
such as `reviewer-output:<bundle-id>:<output-id>`. Parent-only review judgment
cannot clear or fail a review gate.

Parent review loops should capture review truth before launching child reviewer
lanes and submit that snapshot when recording the review outcome. The snapshot
guards the visible mission package and hidden `.ralph/missions/<mission-id>/`
truth so child-lane mutations are detected before any gate is cleared.
The full `review_truth_snapshot` is parent-held writeback capability and is not
embedded in child-visible review evidence snapshots. Child evidence snapshots
carry only a non-capability guard binding that can prove the evidence was
derived from a parent-held truth snapshot.
Child reviewers persist only their bounded `NONE` or structured findings result
with `record-reviewer-output`; that inbox is excluded from review truth drift
checks and does not update gates, ledgers, closeouts, specs, packages, or
mission completion.
When a parent loop lease carries parent loop authority, parent-only mutation
commands such as review-bundle compilation, review-truth capture, review
evidence capture, and review outcome writeback require the parent authority
token. Child reviewers must not receive that token, so they can persist bounded
reviewer output without being able to mint or use parent writeback authority.

The commands below are the stable machine-side helpers those skills can call to
persist mission truth under `PLANS/` and `.ralph/`.

## Command surface

The support CLI also exposes top-level helper commands such as `codex1 setup`,
`doctor`, `qualify-codex`, `restore`, and `uninstall`.

The deterministic backend commands described here live under
`codex1 internal ...` and support `--json`.

`codex1 internal stop-hook` remains the single hook-facing adapter, but the
artifact-driven stop decision now lives in `codex1-core` so the CLI crate does
not carry duplicate Ralph branching logic.

Ralph continuation is lease-scoped. Without an active parent loop lease, normal
parent Stop-hook turns yield with passive pending-work status instead of being
blocked by open mission gates. Generic subagent Stop-hook payloads yield before
resume handling; the parent remains responsible for integrating, retrying, or
marking missing subagent output. Explicit parent loop workflows use the loop
lease commands below to opt into enforcement.

- `codex1 internal begin-loop-lease`
  - input: `RalphLoopLeaseInput`
  - writes `.ralph/loop-lease.json`
  - marks a parent-owned `planning_loop`, `execution_loop`, `review_loop`, or
    `autopilot_loop` as active for one mission
  - requires parent begin authority via `CODEX1_PARENT_LOOP_BEGIN=1`; reviewer
    lanes must not receive this environment capability
  - returns a transient parent loop authority token while persisting only a
    verifier; parent-only mission mutations during that active loop must provide
    the token via `CODEX1_PARENT_LOOP_AUTHORITY_TOKEN`
  - while active, Stop hook blocks on that mission's owed work

- `codex1 internal pause-loop-lease`
  - input: `RalphLoopLeasePauseInput`
  - marks the current loop lease paused so user discussion can stop normally
    without uninstalling or moving hooks
  - this is the backend used by the public `$close` pause surface

- `codex1 internal clear-loop-lease`
  - removes the loop lease and returns the previous lease, if any
  - use when the user explicitly wants to discard the active loop lease rather
    than pause it for later resumption

- `codex1 internal inspect-loop-lease`
  - returns the current loop lease, if any

Public loop skills acquire leases:

- `$plan` uses `planning_loop`
- `$execute` uses `execution_loop`
- `$review-loop` uses `review_loop`
- `$autopilot` uses `autopilot_loop`
- `$clarify` does not acquire a lease during manual intake
- `$close` pauses or clears the active lease

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

- `codex1 internal capture-review-truth-snapshot --mission-id <id> --bundle-id <id>`
  - captures the visible mission package and hidden mission runtime tree before
    child reviewer lanes run
  - returns a `ReviewTruthSnapshot` that parent-owned review writeback can use
    to reject contaminated review waves
  - returns one transient parent writeback token per review bundle while storing
    only a verifier in the repo-visible review truth snapshot
  - refuses to remint parent writeback authority for a bundle that already has a
    captured review truth snapshot; compile a fresh review bundle for a new
    review wave
  - the plaintext token must not be passed to child reviewer prompts or
    child-readable evidence artifacts

- `codex1 internal capture-review-evidence-snapshot --mission-id <id> --bundle-id <id>`
  - writes a frozen reviewer brief under
    `.ralph/missions/<mission-id>/review-evidence-snapshots/`
  - requires an existing parent-held review truth snapshot for the bundle and
    reuses its persisted verifier without minting a replacement token
  - includes bundle bindings, governing fingerprints, proof rows, receipts,
    changed-file refs, reviewer instructions, and a non-capability review truth
    guard binding
  - does not include the full parent-held `review_truth_snapshot` required by
    `record-review-outcome`

- `codex1 internal validate-review-evidence-snapshot --mission-id <id> --bundle-id <id>`
  - rejects frozen reviewer briefs that omit required governing refs, proof
    rows, evidence refs, receipts, changed-file context, or findings-only
    reviewer instructions

- `codex1 internal record-reviewer-output`
  - input: `ReviewerOutputInput`
  - writes one bounded reviewer-output artifact under
    `.ralph/missions/<mission-id>/reviewer-outputs/<bundle-id>/`
  - accepts only `NONE`/`output_kind: "none"` with no findings or structured
    findings with `P0`/`P1`/`P2`/`P3` severity, evidence refs, rationale, and
    suggested next action
  - binds the output to the mission id, review bundle id, reviewer id, and the
    canonical child review evidence snapshot fingerprint
  - returns an evidence ref shaped
    `reviewer-output:<bundle-id>:<output-id>` for parent writeback
  - does not update gates, `REVIEW-LEDGER.md`, per-spec `REVIEW.md`,
    closeouts, specs, packages, state completion, or mission-close artifacts

- `codex1 internal record-review-outcome`
  - legacy alias: `codex1 internal record-review-result`
  - input: `ReviewResultInput`
  - accepts an optional `review_truth_snapshot` captured before child reviewer
    execution and rejects writeback when mission truth drift is detected
  - requires the parent-held writeback authority token returned by
    `capture-review-truth-snapshot`; a persisted repo-visible truth snapshot
    without that token cannot clear a gate
  - requires reviewer-agent output evidence for guarded review writeback, using
    existing inbox refs such as `reviewer-output:<bundle-id>:<output-id>`;
    cited reviewer-output artifacts must be recorded after the parent truth
    snapshot capture so reviewer lanes cannot capture authority after writing
    their own results;
    contaminated waves may route explicitly with
    `review-wave-contaminated:<reason>` instead of pretending parent-local
    judgment reviewed the target
  - rejects reviewer-lane-like writeback identities; reviewer-output evidence
    is review evidence only, not permission for a child reviewer lane to clear
    its own gate, and arbitrary or missing reviewer-output inbox artifacts are
    rejected
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
  - checks readability/summary presence of `README.md`
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
