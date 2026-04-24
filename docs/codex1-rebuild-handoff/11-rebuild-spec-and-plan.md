# 11 Rebuild Spec And Plan

This file turns the handoff into an implementation sequence. It does not replace
the detailed contracts in files `00` through `10`; it points a fresh
implementation agent at the smallest sane path through them.

## One Product

Codex1 is one integrated product:

```text
skills-first UX
deterministic codex1 CLI
visible durable mission files when needed
status --json as the integration surface
Ralph as a thin Stop-hook guard
normal planning for ordinary work
graph planning for large/risky/multi-agent work
planned review boundaries
repair and replan loops
mission-close review
```

The foundation vertical slice is implementation order, not product scope. It
proves the shared substrate before graph/review/replan uses it. Codex1 is not
complete until the integrated graph/review/repair/replan/close behavior works.

## Rebuild Starting Point

The repo may contain only this handoff folder when implementation begins. Treat
that as intentional. Do not search for old source code to preserve. Build from
these docs and the current official Codex source of truth:

```text
/Users/joel/.codex/.codex-official-repo
```

Use official OpenAI/Codex docs when external product guidance is needed. Do not
invent a wrapper runtime around Codex.

## Product Spine

The implementation should make this loop feel simple:

```text
User invokes a skill.
Skill decides chat-only, durable normal, or graph.
CLI records deterministic durable truth when needed.
status --json projects the one next action.
Codex executes until close, interrupt, invalid state, or explain-and-stop.
Ralph only blocks active unpaused autonomous continuation.
```

The user should mostly experience:

```text
$clarify
$plan
$execute
$review-loop
$interrupt
$autopilot
```

The user should not feel like they are operating `STATE.json`.

## Code Shape

Prefer a small core library with thin command wrappers.

Core modules:

```text
mission_resolver
artifact_parser
canonical_digest
state_store
state_transaction
status_projector
stop_projector
plan_checker
task_lifecycle
close_readiness
ralph_adapter
skill_templates
```

Integrated modules that build on the same core:

```text
graph_validator
wave_deriver
review_recorder
finding_adjudicator
repair_budget
replan_lifecycle
mission_close
role_config_writer
doctor_checks
```

Commands should mostly parse args, call shared code, and format output. Do not
let each command re-derive readiness, close gates, or stop behavior.

## Build Sequence

### 1. Project Skeleton

Create the implementation skeleton, packaging, command entrypoint, and test
runner.

Required:

- `codex1 --help`
- `codex1 <subcommand> --help`
- `--json` support
- stable error shape
- installed command can run from outside the source checkout

### 2. Artifact And State Core

Implement:

- `OUTCOME.md` frontmatter parser
- `PLAN.yaml` parser
- `STATE.json` parser
- `EVENTS.jsonl` appender
- `CLOSEOUT.md` parser/writer
- canonical digest rules
- mission-local lock
- atomic state write
- one-event-per-successful-mutation rule

Do this before fancy commands. Boring state correctness is the product's floor.

### 3. Status Spine

Implement `project_status(snapshot) -> status`.

It owns:

- artifact consistency
- digest freshness
- verdict
- phase
- next_action priority
- ready normal steps
- ready graph tasks/waves
- review summary
- replan summary
- close readiness
- stop projection

`codex1 status`, `codex1 task next`, `codex1 close check`, and Ralph must all
use this shared logic.

### 4. Foundation Vertical Slice

Implement the proving path:

- `codex1 init`
- `codex1 doctor --json`
- `codex1 outcome check`
- `codex1 outcome ratify`
- `codex1 plan choose-mode`
- `codex1 plan choose-level`
- `codex1 plan scaffold --mode normal`
- `codex1 plan check`
- `codex1 plan lock`
- `codex1 task next`
- `codex1 task start`
- `codex1 task finish`
- `codex1 loop activate`
- `codex1 loop pause`
- `codex1 loop resume`
- `codex1 loop deactivate`
- `codex1 close check`
- `codex1 close complete`
- `codex1 ralph stop-hook`

Proof path:

```text
$clarify -> OUTCOME.md ratified
$plan -> PLAN.yaml checked and locked
$execute -> all normal steps complete with proof
$interrupt -> loop pauses and Ralph allows stop
$autopilot -> can drive clarify/plan/execute/close through skills
close -> CLOSEOUT.md plus terminal state
```

This slice proves the substrate. It does not redefine Codex1 as a smaller
product.

### 5. Graph Planning

Implement graph plan validation and derived ready frontier/wave projection:

- graph task IDs
- `depends_on`
- specs/proof expectations
- review boundaries
- cycle detection
- missing dependency detection
- superseded/cancelled filtering
- ready frontier
- recommended safe wave
- serial fallback when parallel safety is unclear

Graph waves are derived views, not editable truth.

### 6. Review, Repair, Replan

Implement planned review boundaries:

- `codex1 review start`
- `codex1 review packet`
- `codex1 review record`
- `codex1 review repair-record`
- `codex1 review status`
- `codex1 replan check`
- `codex1 replan record`
- `codex1 plan lock --replan`

Rules:

- Reviewers return findings only.
- Main/root records review truth.
- Raw findings are observations.
- Only accepted blocking findings create repair work.
- `repair_round` increments once per accepted-blocker repair batch.
- Dirty after repair budget triggers autonomous replan.
- Replan relock validates `PLAN.yaml`, refreshes digests, supersedes old work,
  and clears `replan.required`.

### 7. Mission Close

Implement mission-close review and terminalization:

- `codex1 close record-review`
- graph/large/risky mission-close gate
- closeout pre-terminal revision rule
- closeout digest storage
- terminal state
- active pointer clearing

Do not add a public `$finish` or `$complete` skill.

### 8. Skills

Create the actual skill wrappers:

- `$clarify`
- `$plan`
- `$execute`
- `$review-loop`
- `$interrupt`
- `$autopilot`

Skills should read like recipes for Codex, not CLI manuals. They should prefer
`codex1 status --json`, avoid raw ceremony in user-facing prose, and keep the
main/root orchestrator responsible for mission truth.

### 9. Ralph And Codex Integration

Install Ralph through Codex's stable Stop-hook system.

Ralph should:

- parse Stop-hook input
- allow immediately when `stop_hook_active == true`
- resolve mission conservatively
- call the same status/stop projection
- block only known active unpaused autonomous next actions
- fail open for ambiguity, corrupt state, unknown next action, or status error

Custom subagent role configs must set:

```toml
[features]
codex_hooks = false
```

The CLI must not detect parent vs subagent identity.

### 10. Doctor And E2E Proof

Default `doctor` is fast and non-invasive:

- installed command
- useful help
- model policy presence
- hook snippets parse
- Ralph allow/block JSON validity
- state/event drift diagnostics

`doctor --e2e` or equivalent tests prove deeper integration:

- custom subagent role with `codex_hooks = false` does not run Ralph
- main/root orchestrator does run Ralph
- installed command works outside source checkout

## Acceptance Tests

Foundation tests:

- no mission -> status inactive and stop allowed
- outcome ratify stores digest
- outcome edit after ratify -> invalid state
- plan lock stores plan digest and outcome digest at lock
- plan edit after lock -> invalid state
- locked normal plan projects first step
- task start increments revision once
- task finish requires proof
- task finish advances next step
- all steps complete projects close complete
- close complete writes/verifies closeout and terminal state
- paused loop allows stop
- active loop with autonomous next action blocks stop
- `stop_hook_active == true` allows stop
- status and close check agree

Integrated tests:

- graph cycle rejected
- missing dependency rejected
- ready frontier derived from task/review state
- parallel-unsafe wave returns recommended serial task
- planned review task produces review packet
- raw findings do not block before triage
- accepted blocking findings block
- repair-record increments repair round once
- dirty after max repair rounds projects replan
- replan relock appends new task IDs and does not reuse superseded IDs
- stale review does not affect current truth
- mission-close review required before graph terminal close
- Ralph fail-opens for corrupt state and unknown next action

## Stop Conditions

Pause implementation and re-read the handoff if the build starts adding:

- a wrapper runtime around Codex
- hidden hook state
- caller identity enforcement
- reviewer writeback authority
- stored waves
- universal DAG planning
- many cache/gate/receipt files
- multiple readiness engines
- CLI semantic questionnaires

The boring path is the good path: exact state, exact status, clear skills, and
native Codex doing the work.
