# 12 One-Shot Implementation Prompt

This file is the fresh-thread handoff prompt for the next Codex1 implementation
attempt.

Use it in a new worktree or new repository. Its companion file is the
implementation authority:

```text
docs/codex1-rebuild-handoff/CODEX1_SOURCE_OF_TRUTH.md
```

The previous attempts taught the important lesson: Codex1 cannot be built by one
agent filling in a broad command list. It must be built from invariant depth
outward, with tiny owned implementation slices, adversarial tests, and one
canonical source of truth read by every worker.

This prompt tells the new session to orchestrate subagents. It should produce
the CLI and durable substrate, excluding the user-facing Codex skills.

---

# Copy-Paste Prompt Starts Here

You are building Codex1.

Treat this task as mission-critical. The previous implementations failed
because they implemented command breadth before invariant depth. You are not
here to produce a quick skeleton. You are here to produce the strongest,
cleanest, most adversarially tested foundation implementation you can.

## Hard Authority

Before doing anything else, read this file:

```text
docs/codex1-rebuild-handoff/CODEX1_SOURCE_OF_TRUTH.md
```

That file is the implementation authority. Every subagent you spawn must read it
before writing code. If any other doc, review, old implementation, or your memory
disagrees with it, the source-of-truth file wins.

Do not copy old Codex1 code from failed branches or worktrees. You may read old
review findings as cautionary examples only.

Use the local official Codex repo as the source of truth for Codex internals:

```text
/Users/joel/.codex/.codex-official-repo
```

When verifying Stop hooks, hook config, model catalog, or review schema, inspect
that repo. Do not rely on memory.

Use the local CLI creator skill before designing the CLI:

```text
/Users/joel/.codex/skills/cli-creator/SKILL.md
```

Follow its discipline: composable commands, stable JSON, useful help, setup
checks, installed command smoke tests, and a CLI Codex can use from any repo.

Do not write Codex skills in this task. Build the CLI/product substrate that the
skills will call.

## Product Center

Codex1 is one product:

```text
skills-first UX
deterministic codex1 CLI
durable mission files under PLANS/<mission-id>/
status --json as the central integration artifact
Ralph Stop-hook guard over status
normal planning
graph planning
planned review boundaries
repair/replan loops
mission-close review
doctor diagnostics
```

Graph, review, replan, and mission-close review are not a second product and are
not conceptually deferred. You may stage implementation internally, but the
architecture and status model must be designed around the complete product from
the beginning.

The user-facing skills will eventually be:

```text
$clarify
$plan
$execute
$review-loop
$interrupt
$autopilot
```

You are not writing those skills here.

## Your First Output In The New Thread

Do not code immediately.

First produce a short implementation map:

1. language/runtime choice and why it matches this repo
2. module layout
3. subagent ownership plan
4. build gates in order
5. test gates in order
6. how every prior failure class will be prevented

Then start work. Do not wait for approval unless blocked by missing information
that cannot be reasonably inferred from the source-of-truth file.

## Required Orchestration Strategy

You must orchestrate GPT-5.5 subagents with extra-high reasoning for narrowly
owned slices. Use workers for implementation, explorers for focused questions,
and reviewers for adversarial review. Do not ask one subagent to build "the CLI"
or "status" as a vague blob.

Each worker prompt must include:

- "Read `docs/codex1-rebuild-handoff/CODEX1_SOURCE_OF_TRUTH.md` first."
- exact file/module ownership
- explicit interfaces it may rely on or must provide
- exact invariants from the source-of-truth file it must satisfy
- exact regression tests it owns
- "You are not alone in the codebase. Do not revert others' work."
- "Think adversarially about malformed files, stale state, path escapes,
  concurrency, partial commits, and JSON stability."

Each worker must finish with:

- changed files
- tests added
- invariants covered
- remaining risks

The main thread integrates. Workers do not make broad unrelated refactors.

## Subagent Roster

Spawn these workers or equivalent narrower slices. If the language/runtime makes
different filenames natural, keep the ownership boundaries.

### Worker 1: Schemas, Errors, And JSON Envelope

Ownership:

```text
schema/types modules
error/result envelope modules
core enum definitions
unit tests for validation and JSON output shape
```

Responsibilities:

- define stable result and error envelopes
- define canonical enums for verdicts, next actions, loop modes, task statuses,
  review states, error codes
- validate `STATE.json` shape, integer revision, booleans, maps, IDs
- make invalid state data representable as structured errors, not panics
- ensure `--json` parser errors can be emitted as `codex1.error.v1`

Regression focus:

- non-object state
- string booleans
- invalid revision
- noncanonical verdict prevention
- revision conflict shape
- parser error in JSON mode

### Worker 2: Path Resolver And Mission Selection

Ownership:

```text
path/repo/mission resolution modules
ACTIVE.json handling
path security tests
```

Responsibilities:

- mission ID validation
- repo root discovery from `.git`, `PLANS/`, and mission cwd
- resolved-path-under-PLANS invariant
- symlink escape rejection
- `ACTIVE.json` read behavior
- explicit mission requirement helpers for mutating commands

Regression focus:

- `--mission /tmp`
- `--mission ../demo`
- symlinked `PLANS/demo` pointing outside `PLANS`
- non-object `ACTIVE.json`
- stale active pointer
- cwd inside `PLANS/demo`
- non-git repo with `PLANS/`

### Worker 3: Artifact Parsing And Canonical Digests

Ownership:

```text
OUTCOME.md parser
PLAN.yaml parser
proof/review/closeout parsers
canonical digest module
artifact validation tests
```

Responsibilities:

- YAML frontmatter parser for `OUTCOME.md`
- YAML object parser for `PLAN.yaml`
- canonical stable digest from parsed machine object
- proof record validation
- review record validation
- closeout frontmatter validation
- no raw-byte digest false positives from whitespace or key order

Regression focus:

- empty outcome fields
- empty `constraints` and `non_goals`
- semantic digest changes
- whitespace/key-order digest stability
- stale proof digest
- stale closeout frontmatter
- path escape in closeout/proof/review path

### Worker 4: State Store, Locking, Transactions, Events

Ownership:

```text
state_store/transaction modules
atomic write
event append
lock implementation
transaction tests
```

Responsibilities:

- one mutation helper used by all writes
- lock before artifact writes
- atomic `STATE.json` write
- exactly one revision increment
- exactly one event per successful mutation
- event append failure after state commit returns success with warning
- audit drift diagnostics helpers
- `REVISION_CONFLICT` with current revision

Regression focus:

- lock prevents plan scaffold artifact writes
- event append failure with `EVENTS.jsonl` as directory
- missing event log with revised state
- malformed event rows
- concurrent stale revision conflict

### Worker 5: Plan Validator, Graph Validator, Wave Scheduler

Ownership:

```text
plan validation modules
graph validation modules
ready frontier and recommended wave modules
graph tests
```

Responsibilities:

- normal plan validation
- graph plan validation: missing deps, duplicate deps, cycles, IDs, boundaries
- state-plan work item matching helpers
- dependency satisfaction
- ready frontier
- safe recommended wave
- write path conflict detection
- active wave reservation semantics

Regression focus:

- invalid graph plans cannot lock
- dependencies respected
- ready wave excludes exclusive resource conflicts
- ready wave excludes write path conflicts
- starting T1 in safe `[T1,T2]` wave keeps T2 startable
- task outside safe wave cannot start while conflict in progress
- deleting planned task from state projects invalid state

### Worker 6: Status, Stop Projection, And Close Gates

Ownership:

```text
status projector
stop projector
close readiness engine
status/close tests
```

Responsibilities:

- implement the exact status priority order from source-of-truth
- recompute current artifact digests
- detect stale ratified/locked artifacts
- derive work readiness from locked plan plus state
- derive close readiness
- project mission-close review before close completion
- produce safe stop projection for Ralph

Regression focus:

- active pending normal step projects `run_step`
- in-progress projects `finish_task`
- inactive wins before replan/close
- paused wins before replan/close
- invalid state fail-opens
- stale digests invalid state
- keyed review blockers prevent close
- mission-close review projects `close_review`
- close cannot trust mutable close state alone

### Worker 7: Task Lifecycle And Proof Anchoring

Ownership:

```text
task start/finish commands or transition modules
proof import/validation integration
task lifecycle tests
```

Responsibilities:

- start normal steps
- start graph tasks from safe wave only
- maintain active wave semantics
- finish only in-progress step/task
- validate proof anchors against current plan/outcome digests
- validate proof recorded revision against start/current revision
- record proof digest/path safely

Regression focus:

- two-step normal flow S1 -> S2 -> close
- stale proof rejected
- future proof revision rejected
- proof before start rejected
- graph safe wave start behavior
- graph unsafe excluded task rejected

### Worker 8: Review, Repair, And Replan Lifecycle

Ownership:

```text
review command transitions
repair command transitions
replan command transitions
review/replan tests
```

Responsibilities:

- review start
- review record with raw output and adjudication
- require complete adjudication
- store freshness anchors
- accepted blocking -> repair required
- repair rounds
- dirty over budget -> replan required
- replan relock contract

Regression focus:

- partial adjudication cannot pass
- duplicate adjudication invalid
- accepted blocking prevents close
- no raw findings passes only with complete valid record
- persisted review anchors non-null
- repair budget exhausted projects replan
- plain plan lock cannot relock
- `plan lock --replan` requires replan state

### Worker 9: Ralph And Doctor

Ownership:

```text
ralph stop-hook adapter
doctor diagnostics
official Codex repo checks
ralph/doctor tests
```

Responsibilities:

- parse Stop-hook stdin
- use Stop-hook `cwd` for resolution
- allow on `stop_hook_active`
- block only after all safety checks
- fail open on unknown/malformed state/status/next action
- doctor checks official Codex repo for hook schemas/config/model facts
- doctor `ok` false for required check failures
- installed command smoke from outside source checkout

Regression focus:

- Stop-hook cwd resolves active mission
- unknown action fail-open
- loop mode none fail-open
- invalid state fail-open
- block message includes interrupt/pause escape hatch
- doctor required failures make `ok: false`
- doctor does not crash on malformed event/state files

### Worker 10: CLI Adapter, Packaging, And Help

Ownership:

```text
CLI parser and command dispatch
package config
README/help text if present
installed smoke tests
```

Responsibilities:

- command surface from source-of-truth
- global `--json`
- convert parser errors to JSON
- keep commands thin
- ensure every command either performs real transition or stable
  `NOT_IMPLEMENTED` without pretending success
- installable command works outside checkout
- `.DS_Store` ignored/untracked

Regression focus:

- `codex1 --json task start S1` parse error is JSON
- `codex1 --help`
- installed `codex1 status --json` from temp dir
- stale source-local PATH/PYTHONPATH/CARGO path does not fake install success

### Worker 11: Regression Harness And Adversarial Fixtures

Ownership:

```text
integration tests
fixture builders
end-to-end normal and graph flows
regression matrix tracking
```

Responsibilities:

- encode every review finding as a test
- create fixture helpers for missions, outcomes, plans, state, events, proofs,
  reviews
- test full normal durable path
- test graph dependency and wave behavior
- test review/replan/close paths
- test installed command if feasible in CI/local

This worker should not own production logic except small test-only helpers.

## Main Thread Duties

The main thread owns architecture integrity.

You must:

- keep the source-of-truth file open mentally while integrating
- prevent modules from re-deriving status/close/stop logic independently
- keep workers inside ownership boundaries
- review every worker patch before accepting it
- run tests after each major integration gate
- spawn focused reviewer agents after each gate
- reject "green tests" that do not cover the adversarial cases

Do not let the code become one giant file. Do not let commands become the brain.
Do not let mutable state fields become readiness truth.

## Build Gates

### Gate 0: Repo And Runtime Skeleton

Deliver:

- package/build configuration
- CLI entry point
- module layout
- test harness
- `codex1 --help`
- `codex1 --json invalid` returns JSON error

No mission logic yet.

### Gate 1: Schemas, Paths, Artifacts

Deliver:

- mission ID validation
- repo discovery
- symlink-safe mission resolution
- `OUTCOME.md` parser/digest
- `PLAN.yaml` parser/digest
- `STATE.json` validator
- result/error envelopes

Tests must already cover invalid IDs, non-object state/active pointers, stale
digests at parser level, and JSON error shape.

### Gate 2: Transactions And Init

Deliver:

- mission lock
- atomic state writes
- event append
- audit drift warning behavior
- `codex1 init`
- `codex1 doctor` basic diagnostics

Tests must cover lock failure, event append failure after commit, revision
conflict, and missing/malformed events.

### Gate 3: Outcome And Plan Lock

Deliver:

- `outcome check`
- `outcome ratify`
- `plan choose-mode`
- `plan choose-level`
- `plan scaffold`
- `plan check`
- `plan lock`
- graph plan validation
- state route initialization from locked plan

Tests must prove empty outcomes fail, valid outcomes ratify, normal and graph
plans validate, invalid graph plans cannot lock, and plain relock is rejected.

### Gate 4: Status Core And Ralph

Deliver:

- status priority order
- artifact digest freshness checks
- state-plan mismatch checks
- normal and graph ready projection
- close readiness projection
- stop projection
- `ralph stop-hook`

Tests must cover invalid state fail-open, Stop-hook cwd, manual mode none,
pending work block, stale digests, mission-close review gate, and unknown action
fail-open.

### Gate 5: Task And Proof Lifecycle

Deliver:

- `task next`
- `task start`
- `task finish`
- proof validation
- normal two-step execution
- graph active wave start semantics

Tests must cover stale proofs, wave members remaining startable, unsafe excluded
tasks rejected, and all planned work matched against state.

### Gate 6: Review, Repair, Replan, Close

Deliver:

- `review start`
- `review record`
- `review repair-record`
- `replan check`
- `replan record`
- `plan lock --replan`
- `close check`
- `close record-review`
- `close complete`

Tests must cover complete triage, partial triage failure, accepted blockers,
repair budget, replan relock, closeout rewrite, closeout revision semantics, and
terminal state.

### Gate 7: Doctor, Install, End-To-End

Deliver:

- official Codex repo checks
- installed command smoke from temp cwd
- full normal mission e2e
- graph/review/close e2e
- all regression tests

Do not call the product complete before this gate.

## Review Strategy

After each gate, spawn at least one independent reviewer with GPT-5.5
extra-high reasoning. The review prompt must ask for bugs only:

```text
Review this gate against CODEX1_SOURCE_OF_TRUTH.md. Prioritize invariant breaks,
partial commits, path escapes, stale evidence, unsafe Ralph blocks, missing JSON
contracts, graph scheduling mistakes, review/replan loopholes, and close
readiness lies. Do not praise. Return findings with exact files/lines and
reproduction steps.
```

The main thread decides what to accept. Do not blindly implement every review
comment, but every accepted finding must get a regression test.

## Critical Anti-Patterns To Avoid

These are how the previous attempts failed:

- implementing command stubs that say success
- trusting `STATE.close.state` for close readiness
- trusting `STATE.tasks` without comparing to locked `PLAN.yaml`
- lexical path checks instead of canonicalized resolved checks
- writing artifacts before the mission lock
- returning error after a state commit succeeded
- allowing `ACTIVE.json` to select missions for ordinary writes
- letting stale proofs, stale closeouts, or stale reviews complete work
- allowing partial adjudication to pass review
- letting graph plans lock before graph validation is real
- computing `ready_tasks` and calling that a safe wave
- starting graph tasks outside the scheduler-approved wave
- blocking Ralph from unknown or malformed next actions
- letting parser errors escape as plain text in `--json`
- putting everything in one giant file

If you are about to write code that makes one of these possible, stop and fix
the design.

## Concrete Product Behaviors To Preserve

### Execute

`$execute` will later call the CLI to continue an already locked durable plan to
close. It should execute planned review boundaries if they are part of the
locked plan. It stops when close is complete, interrupted, invalid, or the
status projection says no safe autonomous action exists.

### Autopilot

`$autopilot` will later run clarify, plan, execute, review/repair/replan as
needed, and prepare for PR intent according to the ratified outcome. It should
not open a PR unless the clarify/outcome phase explicitly ratifies that user
intent.

### Review Loop

`$review-loop` is an additional explicit pressure loop for repeated review and
fix cycles. It is not the same as planned review boundaries that are already
part of a locked plan. Planned review boundaries are executed by `$execute`.

### Interrupt

`$interrupt` pauses or deactivates active loops so the user can talk without
Ralph pressure.

These skills are not implemented here, but the CLI/status behavior must support
them cleanly.

## Required Schemas

Use the schema shapes and semantics in `CODEX1_SOURCE_OF_TRUTH.md`. If you need
to adjust small implementation details, update tests and keep the same external
contract. Do not invent extra truth fields that duplicate existing truth.

Especially preserve:

- `codex1.result.v1`
- `codex1.error.v1`
- `codex1.status.v1`
- `codex1.outcome.v1`
- `codex1.plan.v1`
- `codex1.state.v1`
- `codex1.event.v1`
- `codex1.proof.v1`
- `codex1.review.v1`
- `codex1.closeout.v1`

## Minimum Final Verification

Run the repo's actual test commands. At minimum, run equivalents of:

```bash
codex1 --help
codex1 --json definitely-invalid-command
codex1 status --json
codex1 doctor --json
```

Also run:

```text
unit tests
integration tests
compile/type/lint checks available in the repo
installed command smoke test from a temporary directory
full normal durable mission e2e
graph dependency/wave e2e
review triage/repair/replan e2e
closeout terminalization e2e
Ralph Stop-hook cwd e2e
```

If a verification step cannot run, say exactly why and what risk remains.

## Final Deliverable

When done, report:

- files/modules created
- command surface implemented
- status/Ralph behavior implemented
- graph/review/replan/close behavior implemented
- regression tests mapped to every prior finding
- verification commands and results
- any remaining intentionally unsupported commands, with stable JSON behavior

The best implementation will feel almost boring: small modules, hard invariants,
thin commands, nasty tests, and no hidden second truth surface.

# Copy-Paste Prompt Ends Here
