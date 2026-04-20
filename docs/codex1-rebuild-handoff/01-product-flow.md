# 01 Product Flow

This file defines the user-facing workflow. A new agent should be able to implement the skills and CLI behavior from these flows without inventing extra phases.

## Skills

Codex1 exposes six public skills:

```text
$clarify
$plan
$execute
$review-loop
$close
$autopilot
```

The user should think in skills, not in CLI commands.

The main Codex thread uses CLI commands behind the scenes.

## Manual Flow

```mermaid
flowchart TD
    User["User describes goal"] --> Clarify["$clarify"]
    Clarify --> Outcome["OUTCOME.md ratified"]
    Outcome --> ChooseLevel["codex1 plan choose-level"]
    ChooseLevel --> Plan["$plan light / medium / hard"]
    Plan --> FullPlan["Full plan: outcome interpretation, architecture, DAG, specs, proof strategy, review tasks"]
    FullPlan --> Waves["codex1 plan waves derives current ready wave"]
    Waves --> Execute["$execute"]
    Execute --> Kind{"Next task kind?"}
    Kind -->|work/design/docs/test/research/repair| Work["Main thread or worker executes assigned task"]
    Kind -->|review| ReviewTask["$review-loop executes planned review task"]
    Work --> Proof["Write task proof"]
    Proof --> Finish["codex1 task finish"]
    Finish --> Waves
    ReviewTask --> ReviewResult{"P0/P1/P2 findings?"}
    ReviewResult -->|No| RecordClean["Main thread records clean review"]
    ReviewResult -->|Yes| RecordFindings["Main thread records findings"]
    RecordFindings --> Dirty{"Six consecutive dirty for same active target?"}
    Dirty -->|No| Repair["Repair through DAG task or current task rerun"]
    Dirty -->|Yes| Replan["$plan replans DAG with new task IDs"]
    Repair --> Waves
    Replan --> Waves
    RecordClean --> More{"More DAG tasks?"}
    More -->|Yes| Waves
    More -->|No| MissionCloseReview["Mission-close review loop"]
    MissionCloseReview --> CloseCheck["codex1 close check"]
    CloseCheck --> Complete["codex1 close complete"]
```

## Autopilot Flow

`$autopilot` composes the whole manual flow.

```mermaid
flowchart TD
    Start["User invokes $autopilot"] --> NeedOutcome{"Ratified OUTCOME.md exists?"}
    NeedOutcome -->|No| Clarify["Run $clarify until outcome is ratified"]
    NeedOutcome -->|Yes| NeedPlan{"Valid PLAN.yaml DAG exists?"}
    Clarify --> NeedPlan
    NeedPlan -->|No| ChooseLevel["Run codex1 plan choose-level"]
    ChooseLevel --> Plan["Run $plan at requested/effective level"]
    NeedPlan -->|Yes| Status["codex1 status --json"]
    Plan --> Status
    Status --> Next{"Next action"}
    Next -->|run task/wave| Execute["Run $execute"]
    Next -->|planned review task| Review["Run $review-loop once"]
    Next -->|repair needed| Repair["Repair or spawn worker"]
    Next -->|replan required| Replan["Run $plan replan"]
    Next -->|mission close| CloseReview["Run mission-close review loop"]
    Execute --> Status
    Review --> Status
    Repair --> Status
    Replan --> Status
    CloseReview --> CloseCheck["codex1 close check"]
    CloseCheck --> CanClose{"Can close?"}
    CanClose -->|No| Status
    CanClose -->|Yes| Complete["codex1 close complete"]
```

`$autopilot` must pause when genuine user input is required. It must not invent user preferences that change scope, risk, money, deployment, destructive actions, or irreversible operations.

When planning is needed, `$autopilot` should either run `codex1 plan choose-level` or use a previously recorded requested level. In a fully autonomous run, the main thread may choose the safest applicable level and record it, but it must still allow effective-level escalation when risk requires it.

## `$clarify`

Purpose:

```text
Create a fully specified OUTCOME.md.
```

What it does:

- Interviews the user.
- Captures the original goal.
- Resolves ambiguity.
- Writes mission destination, must-be-true requirements, success criteria, non-goals, constraints, definitions, quality bar, proof expectations, review expectations, risks, and resolved Q&A.
- Ratifies only when a future Codex thread can understand the mission without hidden chat context.

What it does not do:

- It does not plan unless running inside `$autopilot`.
- It does not start a loop.
- It does not execute work.

## `$plan`

Purpose:

```text
Create a full mission plan with a valid task DAG.
```

What it does:

- Reads `OUTCOME.md`.
- Runs or follows `codex1 plan choose-level`.
- Records the requested planning level.
- Escalates the effective planning level if mission risk requires it.
- Produces architecture/design approach.
- Produces task DAG in `PLAN.yaml`.
- Produces task specs.
- Produces proof strategy.
- Produces planned review tasks.
- Produces mission-close criteria.
- Uses subagents for hard planning when needed.
- Validates with CLI before locking.

What it does not do:

- It does not execute tasks.
- It does not run formal review except plan-review/critique during hard planning.
- It does not store waves as truth.

## `$execute`

Purpose:

```text
Execute the next ready task or ready wave from the DAG.
```

Flow:

```mermaid
flowchart TD
    Start["$execute"] --> Status["codex1 status / task next"]
    Status --> Ready["Ready task or ready wave"]
    Ready --> Safe{"Parallel safe?"}
    Safe -->|Yes| Workers["Spawn workers for wave tasks if useful"]
    Safe -->|No| Single["Run one task serially"]
    Workers --> Proof["Workers report proof"]
    Single --> Proof
    Proof --> Finish["codex1 task finish"]
    Finish --> Next["Return to status"]
```

If the next task is `kind: review`, `$execute` hands to `$review-loop`.

## `$review-loop`

Purpose:

```text
Orchestrate reviewer subagents and record review outcomes.
```

Planned review task mode:

```mermaid
flowchart TD
    Start["Review task ready"] --> Packet["codex1 review packet"]
    Packet --> Spawn["Spawn reviewers with findings-only prompt"]
    Spawn --> Findings["Reviewers return NONE or findings"]
    Findings --> Blocking{"Any P0/P1/P2?"}
    Blocking -->|No| Clean["Main thread records clean"]
    Blocking -->|Yes| Dirty["Main thread records findings"]
    Dirty --> Count["Increment consecutive dirty count"]
    Count --> Replan{"Six consecutive dirty?"}
    Replan -->|No| Repair["Repair"]
    Replan -->|Yes| Plan["Replan"]
```

Mission-close mode:

```text
review -> repair/replan -> review -> repair/replan -> clean
```

Stop after six consecutive dirty mission-close review rounds and replan.

## `$close`

Purpose:

```text
Pause the active loop so the user can talk without Ralph forcing continuation.
```

`$close` is not mission completion.

`$close` is a discussion-mode boundary. When the user invokes it, the main thread should pause the loop, answer/clarify/discuss with the user, and then resume or deactivate only after the user/main thread decides.

Commands:

```bash
codex1 loop pause --json
codex1 loop resume --json
codex1 loop deactivate --json
```

Flow:

```mermaid
flowchart TD
    User["User wants to talk / interrupt"] --> Close["$close"]
    Close --> Pause["codex1 loop pause"]
    Pause --> Ralph["Ralph allows stop"]
    Ralph --> Discuss["User and Codex discuss"]
    Discuss --> Resume{"Resume mission?"}
    Resume -->|Yes| LoopResume["codex1 loop resume"]
    Resume -->|No| Deactivate["codex1 loop deactivate"]
```

## Ralph Flow

Ralph is tiny.

```mermaid
flowchart TD
    Stop["Stop event"] --> Status["Run codex1 status --json"]
    Status --> Active{"loop.active && !loop.paused && stop.allow == false?"}
    Active -->|Yes| Block["Block stop with status.stop.message"]
    Active -->|No| Allow["Allow stop"]
```

Ralph must not inspect plan/review files directly. Ralph must not manage subagents. Ralph must not use `.ralph` mission truth.

Only the active main thread should feel Ralph stop pressure. Worker/reviewer/explorer/advisor subagents should be prompted to complete their bounded job and stop normally. Do not build fake role-detection into Ralph to enforce this; keep Ralph status-only and keep subagent behavior prompt-governed.
