# Codex1 UX Flow Contract

**Status:** product contract.
**Purpose:** define what the user types, what Codex1 does, and what the user sees for the native Codex workflow.
**Source of truth:** complements `docs/codex1-prd.md`; does not replace it.

## 1. Product Shape

Codex1 should feel like native Codex with a stronger mission discipline.

The user-facing surface is:

- `$clarify`
- `$plan`
- `$execute`
- `$review-loop`
- `$autopilot`
- `$close`

The normal manual flow is:

```text
$clarify -> $plan -> $execute
```

The autonomous flow is:

```text
$autopilot
```

The discussion / interruption flow is:

```text
$close
```

`$review-loop` is both automatic and public:

- `$execute` and `$autopilot` invoke it automatically when review is owed.
- The user may also invoke `$review-loop` directly to resolve a review gate or rerun a bounded review/fix/review cycle.
- Child reviewer agents never invoke `$review-loop`.

## 2. Manual Flow

### User starts with `$clarify`

```text
User:
$clarify Build a global Codex1 installer that works across repos.
```

Expected behavior:

- Codex creates or selects the mission package.
- Codex asks one high-leverage question at a time by default.
- Codex reads the repo when repo evidence can answer better than the user.
- Codex records user-stated facts, repo-grounded facts, and Codex inferences separately.
- Codex stops before architecture selection.
- Codex writes `MISSION-STATE.md`.
- Codex ratifies `OUTCOME-LOCK.md` only when the lock rule passes.

Expected user-facing result:

```text
Outcome lock is ready:
PLANS/<mission>/OUTCOME-LOCK.md

Manual handoff: invoke $plan when you want me to plan the route.
```

Manual `$clarify` must not automatically begin `$plan`.

### User continues with `$plan`

```text
User:
$plan
```

or:

```text
User:
$plan make this extremely thorough
```

Expected behavior:

- Codex validates the current outcome lock.
- Codex computes effective planning rigor from user request and mission risk floor.
- Codex runs the required planning methods for that rigor.
- Codex uses bounded subagents/advisor when the rigor and risk justify it.
- Codex writes or updates `PROGRAM-BLUEPRINT.md`.
- Codex writes execution-grade `specs/<id>/SPEC.md` files.
- Codex defines proof rows, review requirements, replan boundaries, and the next execution target.
- Codex packages at least the next selected execution target before planning can complete.

Expected user-facing result:

```text
Program blueprint is ready:
PLANS/<mission>/PROGRAM-BLUEPRINT.md

Next executable target:
spec:<id>

Ready for $execute.
```

If the user asks for a compact plan but the risk floor is high, Codex must say so:

```text
You asked for a compact plan, but this touches global hooks, restore/uninstall,
and user config. I raised planning rigor to thorough/max for safety.
```

### User continues with `$execute`

```text
User:
$execute
```

Expected behavior:

- Codex validates the selected execution package.
- Codex executes only inside the package scope.
- Codex runs proof rows and writes receipts.
- Codex opens or resolves review gates when required.
- Codex invokes `$review-loop` automatically when review is owed.
- Codex repairs local findings when the current spec remains valid.
- Codex replans when the finding breaks the spec, package, blueprint, or lock.
- Codex never declares final completion from execution alone.

Expected user-facing progress:

```text
Executing spec:<id>.
Proof running.
Review owed; invoking $review-loop.
Reviewer lanes complete.
Repairing P2 finding.
Rerunning relevant review lane.
Spec accepted-clean.
```

## 3. Review Flow

### Automatic review inside `$execute`

Review is required at proof-worthy boundaries:

- after code-producing execution slices
- after one spec/phase reaches a completion boundary
- after related specs/phases integrate
- after targeted repair
- before mission close

`$execute` should route into `$review-loop` automatically when any required review boundary is reached.

### Explicit `$review-loop`

```text
User:
$review-loop
```

Expected behavior:

- Parent/orchestrator resolves the active review boundary.
- Parent validates or compiles the review bundle.
- Parent captures parent-held review truth.
- Parent captures child-readable review evidence.
- Parent spawns findings-only reviewer agents.
- Reviewer agents return `NONE` or structured findings.
- Reviewer agents write only bounded reviewer-output artifacts.
- Parent aggregates findings, checks required lanes, and records the review outcome.
- Parent repairs or replans as needed.
- Parent repeats until clean or until the review-loop cap requires replan.

Reviewer agents must not:

- invoke skills
- mutate mission truth
- record review outcomes
- clear gates
- decide mission completion
- acquire Ralph leases

Clean review requires:

- no P0, P1, or P2 findings
- required reviewer lanes answered durably
- no review-wave contamination
- current bundle/gate context is fresh

## 4. Autopilot Flow

```text
User:
$autopilot Build X and run it end-to-end.
```

Expected behavior:

- Autopilot uses the same contracts as manual mode.
- Autopilot clarifies until the mission is lock-ready.
- Autopilot plans until the blueprint/spec/package truth is execution-ready.
- Autopilot executes one packaged target at a time.
- Autopilot invokes `$review-loop` whenever review is owed.
- Autopilot repairs, replans, or waits honestly.
- Autopilot closes only after mission-close review is clean.

Autopilot may continue without new user input only when:

- the next branch is clear from durable mission truth
- the mission is not waiting on a human-only decision
- the active loop lease permits continuation
- no review/replan/user gate blocks progress

Autopilot may self-seal a plan only when:

- the Outcome Lock grants autonomy for the mission
- no human-only decision obligation remains open
- the effective planning rigor has been met
- level-5 planning has a satisfied or explicitly dispositioned advisor
  checkpoint
- blueprint and package truth are fresh after the seal

If those conditions are not met, autopilot must either yield `needs_user` for
human-owned choices or route back to planning/package refresh. It must not
execute from stale package truth.

Autopilot exits with one of:

- `Success`: mission complete, mission-close review clean
- `PageUser`: waiting on a genuine user-only decision
- `HardBlocked`: blocked beyond Codex authority after honest attempts
- `Stale`: previous loop appears dead or interrupted; offer recovery
- `ClosedByUser`: user paused or stopped the loop

## 5. `$close` Flow

```text
User:
$close
```

Expected behavior:

- Codex pauses or clears the active parent loop lease.
- Codex does not repair, review, replan, or close the mission.
- Codex does not resolve gates.
- Codex reports passive status only.
- User can now talk normally.
- Already-running bounded child/reviewer/advisor lanes may finish and persist
  bounded outputs, but the parent does not integrate those outputs while paused.
- `Paused` is not `needs_user`, `hard_blocked`, or `complete`.

Expected user-facing result:

```text
Paused the active Codex1 loop.

Current mission:
PLANS/<mission>/

Current branch:
review_required for spec:<id>

Resume with:
$review-loop
```

Resume is done by invoking:

- `$plan`
- `$execute`
- `$review-loop`
- `$autopilot`

On resume, Codex revalidates package, bundle, artifact, and closeout freshness
before integrating any output that landed while paused.

A CLI fallback such as `codex1 close` may exist for recovery/debug use, but the primary UX is the `$close` skill inside Codex.

## 6. "Actually Also Add Y" Flow

If the user sends a normal message during an active autopilot or execution loop, Codex1 must treat that as a human interrupt boundary.

Example:

```text
User:
actually also add email notifications
```

Expected behavior:

- Codex pauses the active parent loop.
- Codex acknowledges the new input.
- Codex does not keep blindly executing the old branch.
- Codex offers bounded options.

Expected user-facing result:

```text
Paused autopilot.

You said: "actually also add email notifications."

Options:
1. Reopen plan and add this as a new spec.
2. Record it as follow-up and resume the current mission.
3. Cancel/close the current mission.
```

If the user chooses to reopen:

- Codex invokes planning/replan workflow.
- New or updated blueprint/spec truth is written.
- Old package/review truth is invalidated when needed.
- Execution resumes only from fresh package truth.

## 7. Reopen Flow

When the user invokes `$clarify` on an existing locked or closed mission, Codex should not silently overwrite.

Expected prompt:

```text
This repo already has a locked mission:
PLANS/<mission>/OUTCOME-LOCK.md

Choose:
1. View current lock.
2. Reopen with a reason.
3. Start a new mission.
```

Reopen requires:

- a reason
- a new lock revision or mission lineage
- visible replan/reopen record
- invalidation of downstream blueprint/package/review truth when affected

## 8. Mission Close Flow

Codex1 may declare mission completion only after:

- lock satisfied
- blueprint obligations satisfied or honestly descoped
- all active specs accepted-clean or explicitly descoped/followed-up
- mission-level proof rows complete
- required review gates clean
- mission-close review clean
- no unresolved contradictions block completion

User-facing completion should say:

```text
Mission complete.

What changed:
...

Proof:
...

Review:
mission-close review clean

Ready for PR / merge.
```

## 9. User-Facing Principle

The user should mostly experience:

```text
Say what I want.
Answer clarification.
Ask Codex1 to plan.
Ask Codex1 to execute.
Interrupt with $close when I want to talk.
Use $autopilot when I want the whole mission carried.
```

The user should not have to manually reason about `.ralph/`, gates, closeouts, leases, truth snapshots, or package fingerprints during ordinary use.
