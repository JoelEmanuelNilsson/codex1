# Internal Command Taxonomy Proposal

This document proposes a clearer taxonomy for the deterministic
`codex1 internal ...` command surface.

It is intentionally a proposal, not a claim that the current repo has already
finished this rename/split work. The current implemented surface remains
described in [runtime-backend.md](./runtime-backend.md).

## Goals

- Keep the public product surface in skills: `$clarify`, `$plan`, `$execute`,
  `$review`, `$autopilot`.
- Push format-heavy, schema-heavy, append-only state mutation into deterministic
  CLI commands.
- Make command names predictable enough that Codex can inspect `--help` and
  choose the right command without conversational folklore.
- Prevent the CLI from turning into a hidden orchestration runtime or mission
  judgment engine.

## Boundary Rule

The boundary should stay:

- Skills own judgment and workflow.
- Visible artifacts and Ralph closeouts own durable mission truth.
- Internal CLI commands own deterministic transforms, validation, append-only
  writes, and repair support.

The CLI should not decide:

- what the mission means
- whether clarify is sufficient
- which architecture wins
- whether a review finding is substantively correct
- whether the mission is truly complete

## Naming Rule

Keep the current flat `codex1 internal <kebab-case-command>` style for now.

That is lower churn than introducing nested subcommands, and it remains highly
agent-friendly because:

- `codex1 internal --help` stays compact
- each command name carries its family in the prefix verb
- compatibility aliases are easy to preserve during migration

Use the first verb as the family marker:

- `init-*`: bootstrap a new mission
- `materialize-*`: write a coordinated artifact set from already-decided truth
- `compile-*`: compile hidden machine contracts
- `derive-*`: derive a child contract from a parent contract
- `validate-*`: run structural or freshness validation
- `record-*`: persist a workflow result that includes a judged outcome from a skill
- `append-*`: append to an append-only log or ledger
- `resolve-*`: deterministically resolve current state into the next machine answer
- `acknowledge-*`: mark a previously opened waiting/request state as emitted
- `inspect-*`: read-only reporting
- `repair-*`: rebuild or repair cached machine state from higher-authority truth

## Canonical Families

### Runtime Adapter

These commands exist because Codex CLI needs one native stop-hook entrypoint and
one repo-state arbitration path.

- `stop-hook`

### Bootstrap

These commands create the initial mission skeleton.

- `init-mission`

### Plan Materialization

These commands write planning outputs after the skill has already selected the
route.

- `materialize-plan`
- later fine-grained helpers where needed:
  - `write-blueprint`
  - `sync-workstream-specs`
  - `sync-execution-graph`
  - `sync-planning-gates`

### Machine Contract Compilation

These commands compile hidden machine contracts from current visible truth and
governing fingerprints.

- `compile-execution-package`
- `compile-review-bundle`

### Derived Child Contracts

These commands derive one bounded child contract from an already-valid parent
contract.

- `derive-writer-packet`

### Validation

These commands answer "is this structurally valid and fresh against current
truth?"

- `validate-execution-package`
- `validate-writer-packet`
- `validate-review-bundle`
- `validate-mission-artifacts`
- later additions:
  - `validate-gates`
  - `validate-closeouts`
  - `validate-visible-artifacts`
  - `validate-machine-state`
  - `validate-execution-graph`

### Workflow Result Recording

These commands persist a workflow result that was already judged by the skill or
reviewer.

- `record-review-outcome`
- `record-contradiction`

### Append-Only Logs

These commands append to append-only logs or visible running histories.

- `append-closeout`
- `append-replan-log`

### Resume And Waiting Resolution

These commands deterministically resolve open waiting state or resume selection.

- `resolve-resume`
- `open-selection-wait`
- `resolve-selection-wait`
- `clear-selection-wait`
- `acknowledge-selection-request`
- `acknowledge-waiting-request`
- later addition:
  - `resolve-stop-decision`

### Inspection And Repair

These commands are read-only or rebuild cached machine state from authoritative
artifacts.

- `inspect-effective-config`
- `repair-state`
- later additions:
  - `latest-valid-closeout`
  - `inspect-gates`

## Current-To-Proposed Mapping

| Current command | Proposed canonical command | Action | Rationale |
| --- | --- | --- | --- |
| `stop-hook` | `stop-hook` | keep | Stable hook entrypoint; bound to Codex hook integration. |
| `rebuild-state` | `repair-state` | rename | The command repairs cached state from higher-authority truth; "repair" is clearer than generic rebuild. |
| `validate-artifacts` | `validate-mission-artifacts` | rename + keep as umbrella | Current name is vague; the command validates the mission artifact set, not arbitrary repo artifacts. |
| `effective-config` | `inspect-effective-config` | rename | Read-only inspector naming should say `inspect-*`. |
| `init-mission` | `init-mission` | keep | Clear bootstrap command with acceptable breadth. |
| `write-blueprint` | `materialize-plan` | rename + later split | Current command writes blueprint, specs, graph, gates, and closeout; the current name understates its scope. |
| `compile-execution-package` | `compile-execution-package` | keep | Precise and already aligned with the PRD. |
| `validate-execution-package` | `validate-execution-package` | keep | Precise and already aligned with the PRD. |
| `compile-review-bundle` | `compile-review-bundle` | keep | Precise and already aligned with the PRD. |
| `derive-writer-packet` | `derive-writer-packet` | keep | Correctly signals a child contract derived from a passed package. |
| `validate-writer-packet` | `validate-writer-packet` | keep | Precise and already aligned with the PRD. |
| `validate-review-bundle` | `validate-review-bundle` | keep | Precise and already aligned with the PRD. |
| `record-review-result` | `record-review-outcome` | rename + later split | The command does more than save a raw result; it records a judged outcome across ledger, gates, and closeout. |
| `record-contradiction` | `record-contradiction` | keep | Good fit for a record command with structured input. |
| `write-replan-log` | `append-replan-log` | rename | The log is append-oriented; the verb should say so. |
| `open-selection-wait` | `open-selection-wait` | keep | Clear and deterministic. |
| `resolve-resume` | `resolve-resume` | keep | This is true state resolution, not generic inspection. |
| `resolve-selection-wait` | `resolve-selection-wait` | keep | Clear and deterministic. |
| `consume-selection` | `clear-selection-wait` | rename | "Consume" is implementation-shaped; "clear" better describes the state transition. |
| `acknowledge-waiting-request` | `acknowledge-waiting-request` | keep | Good acknowledgment verb and specific noun. |
| `acknowledge-selection-request` | `acknowledge-selection-request` | keep | Good acknowledgment verb and specific noun. |
| `write-closeout` | `append-closeout` | rename | The closeout log is append-only; the name should encode that constraint. |

## Commands That Should Be Split

### `write-blueprint` -> `materialize-plan`

Current scope is too broad for the name. It writes:

- `PROGRAM-BLUEPRINT.md`
- frontier `SPEC.md` files and support files
- `execution-graph.json`
- planning gate state
- planning closeout

Recommended direction:

- rename the current high-level command to `materialize-plan`
- later introduce lower-level helpers only if the skills need finer control:
  - `write-blueprint`
  - `sync-workstream-specs`
  - `sync-execution-graph`
  - `sync-planning-gates`

Reason:

- "write blueprint" sounds like one file write
- the implemented behavior is "materialize the planning package"

### `record-review-result` -> `record-review-outcome`

Current scope is also broader than the name. It updates:

- `REVIEW-LEDGER.md`
- per-spec `REVIEW.md`
- matching review gate state
- review closeout

Recommended direction:

- rename the current wrapper command to `record-review-outcome`
- later split only if review workflows need independent machine steps:
  - `write-review-ledger`
  - `update-review-gates`
  - `append-closeout`

Reason:

- the CLI should persist a judged review outcome
- the skill or reviewer still owns the judgment itself

### `validate-artifacts` -> `validate-mission-artifacts`

Current name is too generic for a report that spans visible artifacts and hidden
machine state.

Recommended direction:

- rename to `validate-mission-artifacts`
- add narrower validators as the surface grows:
  - `validate-visible-artifacts`
  - `validate-machine-state`
  - `validate-execution-graph`
  - `validate-gates`
  - `validate-closeouts`

Reason:

- the umbrella validator remains useful for qualification and quick diagnosis
- narrower validators give skills and repair flows more precise tools

### `stop-hook`

Keep the command name stable, but split the logic conceptually.

Recommended internal structure:

- `resolve-resume`
- `resolve-stop-decision` (new deterministic helper)
- `acknowledge-selection-request`
- `acknowledge-waiting-request`

Reason:

- `stop-hook` must remain the one hook-facing adapter command
- the actual state rules inside it should stay small, inspectable, and reusable

## Commands To Add

These additions would make the taxonomy cleaner and reduce overloaded wrappers.

### High priority

- `validate-gates`
  - validate `gates.json` independently of broader artifact validation
- `validate-closeouts`
  - validate NDJSON shape, sequence monotonicity, and crash-tail handling
- `latest-valid-closeout`
  - deterministic resolver for the last schema-valid fully written closeout
- `resolve-stop-decision`
  - pure deterministic stop/yield/continue decision from current Ralph truth

### Medium priority

- `validate-visible-artifacts`
  - validate `README.md`, `MISSION-STATE.md`, `OUTCOME-LOCK.md`,
    `PROGRAM-BLUEPRINT.md`, and `SPEC.md` structure
- `validate-machine-state`
  - validate `state.json`, `active-cycle.json`, and mission-local hidden files
- `validate-execution-graph`
  - pull graph validation out of the umbrella report when direct use becomes
    common

## Commands To Keep Flat Versus Make Hierarchical

Do not move to nested commands yet.

Preferred:

```text
codex1 internal compile-execution-package
codex1 internal derive-writer-packet
codex1 internal append-closeout
```

Not preferred yet:

```text
codex1 internal compile execution-package
codex1 internal derive writer-packet
codex1 internal append closeout
```

Reasons:

- lower migration cost
- simpler `--help`
- easier alias support
- current repo already uses the flat pattern

## Migration Plan

### Phase 1: Canonical names plus aliases

- Add the new canonical names.
- Keep the current names as compatibility aliases.
- Update docs and skills to prefer the canonical names.

### Phase 2: Split overloaded wrappers

- Introduce lower-level helpers behind `materialize-plan` and
  `record-review-outcome`.
- Keep the wrappers for normal skill use.
- Use the finer commands only where the skill truly benefits from them.

### Phase 3: Tighten validation and repair surface

- Add `validate-gates`, `validate-closeouts`, `latest-valid-closeout`, and
  `resolve-stop-decision`.
- Keep `validate-mission-artifacts` as the umbrella command for qualification
  and debugging.

## Recommended End State

The internal CLI should feel like a deterministic compiler/notary layer:

- skills decide
- commands compile or validate
- commands append state in machine-checkable form
- hooks call one deterministic stop adapter

It should not feel like:

- a second runtime
- a hidden mission brain
- a helper shell that secretly owns Ralph

That is the intended balance: more bespoke CLI power, without reintroducing the
forbidden hidden workflow engine.
