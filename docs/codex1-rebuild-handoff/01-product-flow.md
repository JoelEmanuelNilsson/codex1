# 01 Product Flow

This file defines the user-facing workflow. A new agent should be able to implement the skills and CLI behavior from these flows without inventing extra phases.

## Skills

Codex1 exposes six public skills:

```text
$clarify
$plan
$execute
$review-loop
$interrupt
$autopilot
```

The user should think in skills, not in CLI commands.

The main Codex thread uses CLI commands behind the scenes.

There is no public `$finish` or `$complete` skill. Completion is an internal workflow/CLI state reached when the relevant checks pass.

## Planning Modes

Codex1 has two planning modes.

```text
normal = lightweight planning for ordinary work; can be chat-only or durable
graph  = explicit task graph for large/risky/multi-agent work
```

Normal mode is not a weaker version of graph mode. It is the correct mode when a checklist, acceptance criteria, and validation strategy are enough.

Graph mode is for work where dependency order, parallel delegation, review timing, stale outputs, repair boundaries, or terminal proof need machine-checkable structure.

Planning level may still express depth:

```text
light
medium
hard
```

The mode and level are related but not identical. Most normal work is `medium`. Most graph work is `hard`. A small safe task can use normal mode with `light` planning and skip durable mission state entirely.

## Manual Flow

```mermaid
flowchart TD
    User["User describes goal"] --> ClarifyNeeded{"Need durable outcome?"}
    ClarifyNeeded -->|No durable state needed| DirectNormal["Normal work directly"]
    DirectNormal --> DirectProof["Run proportional check"]
    DirectProof --> DirectDone["Summarize result"]

    ClarifyNeeded -->|Yes| Clarify["$clarify"]
    Clarify --> Outcome["OUTCOME.md ratified"]
    Outcome --> ChooseMode["$plan chooses normal or graph"]

    ChooseMode -->|normal| NormalPlan["Normal plan: goal, constraints, checklist, acceptance, validation"]
    NormalPlan --> NormalLock["codex1 plan lock"]
    NormalLock --> NormalActivate["codex1 loop activate"]
    NormalActivate --> NormalExecute["$execute next step"]
    NormalExecute --> NormalCheck["Run checks / inspect diff"]
    NormalCheck --> NormalReview{"Findings or mismatch?"}
    NormalReview -->|No| NormalDone["Internal completion"]
    NormalReview -->|Yes| NormalRepair["Repair or replan"]
    NormalRepair --> NormalExecute

    ChooseMode -->|graph| GraphPlan["Graph plan: tasks, dependencies, specs, proof, review tasks"]
    GraphPlan --> GraphLock["codex1 plan lock"]
    GraphLock --> GraphActivate["codex1 loop activate"]
    GraphActivate --> Waves["codex1 plan waves derives ready wave"]
    Waves --> Execute["$execute"]
    Execute --> Kind{"Next task kind?"}
    Kind -->|work/design/docs/test/research| Work["Main thread or worker executes assigned task"]
    Kind -->|review| ReviewTask["$review-loop executes planned review task"]
    Work --> Proof["Write task proof"]
    Proof --> Finish["codex1 task finish"]
    Finish --> Waves
    ReviewTask --> ReviewResult["Reviewers return NONE or raw findings"]
    ReviewResult --> Triage["Main thread triages findings"]
    Triage --> Accepted{"Accepted blockers?"}
    Accepted -->|No| RecordClean["Main thread records clean review"]
    Accepted -->|Yes| Budget{"Repair budget remains?"}
    Budget -->|Yes| Repair["Repair accepted blockers"]
    Budget -->|No| Replan["$plan replans graph with new task IDs"]
    Repair --> ReReview["Targeted re-review of same boundary"]
    ReReview --> Triage
    Replan --> Waves
    RecordClean --> More{"More graph tasks?"}
    More -->|Yes| Waves
    More -->|No| MissionCloseReview["Mission-close review loop"]
    MissionCloseReview --> RecordCloseReview["codex1 close record-review"]
    RecordCloseReview --> CloseCheck["codex1 close check"]
    CloseCheck --> Complete["codex1 close complete"]
```

## Autopilot Flow

`$autopilot` composes the manual flow.

```mermaid
flowchart TD
    Start["User invokes $autopilot"] --> Status["codex1 status --json if mission exists"]
    Status --> NeedOutcome{"Durable OUTCOME.md needed and ratified?"}
    NeedOutcome -->|No durable mission needed| DirectNormal["Do normal work directly"]
    DirectNormal --> DirectProof["Run proportional proof"]
    DirectProof --> DirectDone["Summarize result"]
    NeedOutcome -->|Needed but missing| Clarify["Run $clarify until outcome is ratified"]
    NeedOutcome -->|Yes| NeedPlan{"Valid plan exists?"}
    Clarify --> NeedPlan
    NeedPlan -->|No| Plan["Run $plan: choose, check, and lock plan"]
    NeedPlan -->|Yes| NextStatus["codex1 status --json"]
    Plan --> Activate["codex1 loop activate if durable loop should continue"]
    Activate --> NextStatus
    NextStatus --> Next{"Next action"}
    Next -->|normal step| ExecuteNormal["Run $execute"]
    Next -->|graph task/wave| ExecuteGraph["Run $execute"]
    Next -->|planned review task| Review["Run $review-loop once"]
    Next -->|repair needed| Repair["Repair or spawn worker"]
    Next -->|replan required| Replan["Run $plan replan"]
    Next -->|mission close| CloseReview["Run mission-close review loop"]
    ExecuteNormal --> NextStatus
    ExecuteGraph --> NextStatus
    Review --> NextStatus
    Repair --> NextStatus
    Replan --> NextStatus
    CloseReview --> RecordCloseReview["codex1 close record-review"]
    RecordCloseReview --> CloseCheck["codex1 close check"]
    CloseCheck --> CanClose{"Can close?"}
    CanClose -->|No| NextStatus
    CanClose -->|Yes| Complete["codex1 close complete"]
```

`$autopilot` must pause when genuine user input is required. It must not invent
user preferences that change scope, risk, money, deployment, irreversible
external operations, or non-Git-managed destructive actions. Version-controlled
repo edits inside the locked mission scope or assigned write paths are
autonomous after mission lock, but Codex1 must not overwrite user work or
silently broaden file ownership when the safe scope is unclear.

When planning is needed, `$autopilot` may choose the lightest safe mode and level, record the decision, and escalate if risk requires it.

## `$clarify`

Purpose:

```text
Create a specified enough target to build the right thing.
```

What it does:

- Interviews the user only for uncertainty that cannot be discovered or safely inferred.
- Captures the original goal.
- Resolves ambiguity that changes product outcome, scope, risk, money, irreversible actions, account access, deployment, privacy, or security.
- Writes mission destination, must-be-true requirements, success criteria, non-goals, constraints, definitions, quality bar, proof expectations, review expectations, risks, and resolved Q&A when durable state is needed.
- Ratifies only when a future Codex thread can understand the mission without hidden chat context.

What it does not do:

- It does not ask questions for ordinary implementation details Codex can safely decide.
- It does not plan unless running inside `$autopilot`.
- It does not start a loop.
- It does not execute work.

## `$plan`

Purpose:

```text
Create the lightest plan that preserves intent and makes execution correctable.
```

What it does:

- Reads the user request or `OUTCOME.md`.
- Chooses `normal` or `graph`.
- Records `planning_mode` for durable missions.
- Records requested/effective planning level when useful.
- Escalates the effective level or mode if mission risk requires it.
- Produces architecture/design approach only as much as the task needs.
- Produces acceptance criteria and validation strategy.
- Produces a normal checklist for normal mode.
- Produces task graph, specs, proof strategy, and planned review tasks for graph mode.
- Uses the single explorer role when missing facts materially affect the plan.
- Uses advisors/critique/reviewers for graph/hard planning when useful.
- Validates with CLI before locking durable plans.
- Runs `codex1 plan lock` for durable plans once the plan is valid.

What it does not do:

- It does not execute tasks.
- It does not turn every mission into a graph.
- It does not run formal review except plan-review/critique during graph/hard planning.
- It does not store waves as truth.

## `$execute`

Purpose:

```text
Execute the next ready step, task, or wave.
```

Normal-mode flow:

```mermaid
flowchart TD
    Start["$execute"] --> Status["codex1 status / plan state"]
    Status --> Activate["Activate durable loop if locked and inactive"]
    Activate --> Step["Next checklist step"]
    Step --> Work["Main thread or bounded worker executes"]
    Work --> Proof["Run proportional proof"]
    Proof --> Update["Record progress"]
    Update --> Next["Return to status"]
```

Graph-mode flow:

```mermaid
flowchart TD
    Start["$execute"] --> Status["codex1 status / task next"]
    Status --> Activate["Activate durable loop if locked and inactive"]
    Activate --> Ready["Ready task or ready wave"]
    Ready --> Safe{"Parallel safe?"}
    Safe -->|Yes| Workers["Spawn workers for wave tasks if useful"]
    Safe -->|No| Single["Run one task serially"]
    Workers --> Proof["Workers report proof"]
    Single --> Proof
    Proof --> Finish["codex1 task finish"]
    Finish --> Next["Return to status"]
```

If the next graph task is `kind: review`, `$execute` hands to `$review-loop`.

## `$review-loop`

Purpose:

```text
Compare implementation evidence against intent, then repair or finish.
```

Normal mode:

- Use a soft review loop.
- For small/local work, main thread inspection plus proportional checks are enough.
- Run tests/checks relevant to touched behavior.
- Inspect diff against the plan and acceptance criteria.
- Use a reviewer subagent only when risk, ambiguity, or blast radius justifies it.
- Repair local issues directly.
- Replan only when the plan no longer matches reality or repeated repair does not converge.

Graph planned review task mode:

```mermaid
flowchart TD
    Start["Review task ready"] --> Packet["codex1 review packet"]
    Packet --> Spawn["Spawn reviewers with findings-only prompt"]
    Spawn --> Findings["Reviewers return NONE or findings"]
    Findings --> Triage["Main thread triages findings"]
    Triage --> Blocking{"Accepted blockers?"}
    Blocking -->|No| Clean["Main thread records clean"]
    Blocking -->|Yes| Budget{"Repair round < 2?"}
    Budget -->|Yes| Repair["Repair accepted blockers"]
    Budget -->|No| Plan["Autonomous replan"]
```

Review findings should include priority and confidence using official Codex-style fields:

```json
{
  "priority": 1,
  "confidence_score": 0.82
}
```

The review event as a whole should include `overall_confidence_score`.

Mission-close mode:

```text
review -> repair/replan -> review -> repair/replan -> clean
```

For graph/large/risky missions, stop repairing a boundary after the repair
budget is exhausted and replan autonomously. The default repair budget is two
repair rounds for the same current review boundary.

Review findings are observations, not work. Only accepted blocking findings can
block progress. The canonical details are in
`07-review-repair-replan-contract.md`.

## `$interrupt`

Purpose:

```text
Pause the active loop so the user can talk without Ralph forcing continuation.
```

`$interrupt` is not mission completion.

`$interrupt` is a discussion-mode boundary. When the user invokes it, the main thread should pause the loop, answer/clarify/discuss with the user, and then resume or deactivate only after the user/main thread decides.

Commands:

```bash
codex1 loop pause --json
codex1 loop resume --json
codex1 loop deactivate --json
```

Flow:

```mermaid
flowchart TD
    User["User wants to talk / interrupt"] --> Interrupt["$interrupt"]
    Interrupt --> Pause["codex1 loop pause"]
    Pause --> Ralph["Ralph allows stop"]
    Ralph --> Discuss["User and Codex discuss"]
    Discuss --> Resume{"Resume mission?"}
    Resume -->|Yes| LoopResume["codex1 loop resume"]
    Resume -->|No| Deactivate["codex1 loop deactivate"]
```

## Ralph Flow

Ralph is minimal.

The canonical Ralph contract is in `06-ralph-stop-hook-contract.md`. If this
section and file `06` disagree, file `06` wins.

Implementation:

```text
Ralph is a Codex Stop hook.
The Stop hook runs a small codex1 hook adapter.
The adapter gets status from the same projection as codex1 status --json.
```

```mermaid
flowchart TD
    Stop["Stop event"] --> Active{"stop_hook_active?"}
    Active -->|true| Allow["Allow stop"]
    Active -->|false| Status["Run codex1 status --json"]
    Status --> Decision{"status.stop.allow?"}
    Decision -->|true| Allow["Allow stop"]
    Decision -->|false| Message{"status.stop.message present?"}
    Message -->|no| Allow
    Message -->|yes| SafeNext{"known required autonomous codex next action?"}
    SafeNext -->|yes| Block["Block stop with status.stop.message"]
    SafeNext -->|no| Allow
```

Tier behavior:

- No active mission: `stop.allow = true`.
- Normal mode: fail open unless there is a valid active unpaused mission with a known autonomous next action.
- Graph mode: block active unpaused loops when the next action is autonomous and safe to continue.
- `invalid_state`, missing mission, corrupt state, paused loop, unknown next action, or `stop_hook_active == true` should allow stop.

Ralph must not inspect plan/review files directly. Ralph must not manage subagents. Ralph must not use `.ralph` mission truth.

Ralph must not depend on `PreToolUse` or `PostToolUse` hooks to reconstruct mission state. Modern Codex hooks can observe MCP tools, `apply_patch`, and long-running Bash sessions; that is useful for optional append-only audit or proof capture, but not for stop authority.

For long-running Bash sessions, Ralph should rely on `codex1 status --json` to know whether the active mission wants continuation. Post-tool hooks may observe completion later, but a running process is not itself mission truth.

Only the active main/root orchestrator should feel Ralph stop pressure.
Worker/reviewer/explorer/advisor subagents should use custom role configs with
Codex hooks disabled:

```toml
[features]
codex_hooks = false
```

Do not build fake role-detection into Ralph to enforce this; keep Ralph
status-only and keep subagent behavior role-config/prompt-governed. Do not use
full-history forks for these custom-role subagents; use explicit task packets.
