# 06 Ralph Stop-Hook Contract

This file is canonical for Ralph.

Ralph is the Codex1 Stop-hook guard. It exists to prevent the main Codex
orchestrator from accidentally stopping while an active mission still has a
clear, autonomous, Codex-owned next action.

Ralph must be helpful, not carceral:

```text
A broken guardrail should not trap the user in Codex.
Ralph should be a helpful interruption layer, not a jail.
```

## Source-Of-Truth Boundary

Ralph is a thin adapter over deterministic Codex1 status.

Ralph may:

- Read Codex Stop-hook JSON from stdin.
- Use `cwd` only to locate the repository/mission root.
- Run the same status projection as `codex1 status --json`.
- Read only the `status.stop` projection, the safety fields in
  `status.next_action`/`next_action` (`kind`, `owner`, `required`,
  `autonomous`), plus Codex's `stop_hook_active` flag.
- Emit official Codex Stop-hook output.

Ralph must not:

- Inspect `PLAN.yaml`, `STATE.json`, reviews, proofs, or closeout directly.
- Parse `last_assistant_message` as mission truth.
- Reconstruct mission state from PreToolUse/PostToolUse hooks.
- Manage subagents.
- Judge implementation quality.
- Decide whether review findings are valid.
- Detect whether the caller is a parent, worker, reviewer, explorer, or advisor.
- Use hidden `.ralph` state.

The CLI owns the stop projection. Ralph only adapts that projection to Codex's
Stop-hook protocol.

## Mission Discovery From `cwd`

Codex Stop hooks provide `cwd`, not a Codex1 mission id. Ralph must use `cwd`
only to select which mission status projection to read. Mission truth still
lives in `PLANS/<mission-id>/`.

Resolution order:

1. If `cwd` is inside `PLANS/<mission-id>/`, use that mission directly.
2. Otherwise find the nearest ancestor project/root containing `PLANS/`.
3. Read `PLANS/ACTIVE.json`.
4. Validate it only as selector metadata:
   - `schema_version == "codex1.active.v1"`.
   - `mission_id` is present.
   - `PLANS/<mission-id>/STATE.json` exists.
   - `STATE.json.mission_id` matches the pointer.
5. Run the shared `codex1 status --json` projection for that mission.
6. If any step is missing, stale, invalid, ambiguous, or surprising, allow stop.

`PLANS/ACTIVE.json` is visible pointer metadata, not mission truth. It must not
contain phase, loop, task, review, close, or terminal state.

Recommended shape:

```json
{
  "schema_version": "codex1.active.v1",
  "mission_id": "codex1-rebuild",
  "selected_at": "2026-04-24T10:00:00Z",
  "selected_by": "codex1 loop activate",
  "purpose": "ralph_status_default"
}
```

Do not scan all missions and guess. Multiple missions with no valid explicit
selection allow stop.

## Official Stop-Hook Facts

Codex Stop-hook input includes:

```json
{
  "session_id": "...",
  "turn_id": "...",
  "cwd": "...",
  "transcript_path": "... or null",
  "hook_event_name": "Stop",
  "model": "...",
  "permission_mode": "...",
  "stop_hook_active": false,
  "last_assistant_message": "... or null"
}
```

The preferred blocking output is:

```json
{
  "decision": "block",
  "reason": "Run wave W2: T2, T3. Use $interrupt to pause."
}
```

Empty stdout or `{}` allows stop. Invalid JSON allows stop. A
`decision:block` response without a non-empty `reason` does not produce a valid
block. Ralph should deliberately fail open in those cases.

## One-Block Rule

Ralph should block at most once per Codex Stop-hook continuation cycle.

Do not use wall-clock timing. Do not write local Ralph memory. Use Codex's
official Stop-hook input:

```text
if stop_hook_active == true:
    allow stop
```

Meaning:

- First stop attempt in a turn, with clear required autonomous work: Ralph may block.
- Second stop attempt in the same hook continuation: Ralph allows stop.

This gives Ralph teeth without creating an infinite loop if Codex ignores or
cannot satisfy the first continuation prompt.

## Main Decision Rule

Ralph blocks only when the status projection and Stop-hook input make all of
these true:

```text
stop_hook_active == false
status.ok == true
mission exists
loop.active == true
loop.paused == false
status.stop.allow == false
status.stop.message is non-empty
next_action.owner == codex
next_action.autonomous == true
next_action.required == true
next_action.kind is on the block allowlist
```

The mission/loop conditions should be encoded by `status.stop`. Ralph should
not independently recalculate mission state from files. Before turning
`status.stop.allow == false` into a Codex Stop-hook block, Ralph may verify
`status.stop.message` is present and the next-action safety fields (`kind`,
`owner`, `required`, `autonomous`) are safe.

Everything else allows stop.

This includes:

- No active mission.
- Missing mission root.
- Ambiguous mission root.
- Missing `PLANS/ACTIVE.json` when `cwd` is not inside a mission.
- Invalid, stale, deleted, or mismatched `PLANS/ACTIVE.json`.
- `codex1 status --json` failure.
- Invalid status JSON.
- Schema mismatch.
- Corrupt or unreadable state.
- Inactive loop.
- Paused loop.
- Mission complete.
- Unknown next action.
- Missing stop message.
- `stop_hook_active == true`.

## Block Allowlist

Ralph should only block on known autonomous next-action kinds:

```text
run_step
run_wave
finish_task
run_review
triage_review
repair
replan
close_review
close_complete
```

Unknown next-action kinds allow stop. Unknown means "do not jail."

`explain_and_stop` is intentionally absent from the block allowlist. It is the
clean non-autonomous stop projection for cases where Codex should explain why no
autonomous continuation exists.

There is intentionally no `validation_required` next-action kind. Proof and
validation requirements are part of `finish_task`, `review`, or `close` checks.

There is intentionally no `blocked_external` stop path. Post-lock Codex1 should
not invent helplessness states for ordinary engineering trouble. Engineering
trouble is handled by repair or replan.

## Decision Table

| Condition | Decision | Reason code |
| --- | --- | --- |
| Cannot parse Stop stdin | allow | `allow_invalid_hook_input` |
| `stop_hook_active == true` | allow | `allow_hook_already_active` |
| Cannot locate mission root | allow | `allow_no_mission_root` |
| Missing active pointer outside a mission dir | allow | `allow_no_active_pointer` |
| Active pointer invalid, stale, or mismatched | allow | `allow_stale_active_pointer` |
| Multiple active missions and no explicit mission | allow | `allow_ambiguous_mission` |
| `codex1 status --json` fails or times out | allow | `allow_status_error` |
| Status JSON invalid | allow | `allow_invalid_status` |
| Status schema unsupported | allow | `allow_schema_mismatch` |
| State corrupt or unreadable | allow | `allow_invalid_state` |
| No active mission | allow | `allow_no_active_mission` |
| Loop inactive | allow | `allow_loop_inactive` |
| Loop paused | allow | `allow_paused` |
| Mission complete | allow | `allow_complete` |
| Unknown or non-autonomous next action | allow | `allow_no_autonomous_next_action` |
| Active normal step remains | block | `block_active_normal_step` |
| Active graph wave remains | block | `block_active_graph_wave` |
| Task needs proof/finish | block | `block_finish_task_required` |
| Required review is ready | block | `block_review_required` |
| Current review findings need triage | block | `block_review_triage_required` |
| Accepted blockers need repair within budget | block | `block_repair_required` |
| Review boundary exhausted repair budget | block | `block_replan_required` |
| Replan has been marked required | block | `block_replan_required` |
| Mission-close review is ready | block | `block_mission_close_review_ready` |
| Mission-close review passed but terminal close not complete | block | `block_close_complete_required` |

## Message Shape

Messages are instructions to Codex, not scolding.

Every block message should:

- Name the exact next action.
- Stay short.
- Include the intentional stop escape hatch.
- Mention the one-block override.
- Avoid quality speculation.

Template:

```text
Codex1 says required work remains: <NEXT_ACTION>.
Continue that now, or use $interrupt / codex1 loop pause to stop intentionally.
If this is a false positive, explain briefly and stop; Ralph will not block again in this turn.
```

Examples:

```text
Codex1 says required work remains: run wave W2: T2, T3.
Continue that now, or use $interrupt / codex1 loop pause to stop intentionally.
If this is a false positive, explain briefly and stop; Ralph will not block again in this turn.
```

```text
Codex1 says required work remains: repair accepted blockers for RB-T4.
Continue that now, or use $interrupt / codex1 loop pause to stop intentionally.
If this is a false positive, explain briefly and stop; Ralph will not block again in this turn.
```

```text
Codex1 says required work remains: complete terminal close.
Continue that now, or use $interrupt / codex1 loop pause to stop intentionally.
If this is a false positive, explain briefly and stop; Ralph will not block again in this turn.
```

## Status Stop Shape

`codex1 status --json` should expose a stop projection like this:

```json
{
  "stop": {
    "allow": false,
    "reason": "block_active_graph_wave",
    "mode": "strict",
    "message": "Codex1 says required work remains: run wave W2: T2, T3.\nContinue that now, or use $interrupt / codex1 loop pause to stop intentionally.\nIf this is a false positive, explain briefly and stop; Ralph will not block again in this turn."
  }
}
```

For allow states:

```json
{
  "stop": {
    "allow": true,
    "reason": "allow_paused",
    "mode": "open",
    "message": null
  }
}
```

Allowed-but-interesting diagnostics may include a message for humans, but Ralph
should normally emit empty stdout on allow.

## Subagents Must Not Feel Ralph

Only the main/root orchestrator should have Ralph Stop-hook pressure.

Do not solve this by teaching Ralph to identify subagents. That would create
fake authority logic in the wrong layer.

Instead:

- Install Ralph in the main/root profile or project configuration.
- Create custom subagent roles for worker, reviewer, explorer, and advisor.
- In every custom subagent role config, disable Codex hooks:

```toml
[features]
codex_hooks = false
```

This follows the same principle used by Codex's own isolated review sessions:
review-like subagents should not inherit general hook pressure from the parent.
The exact e2e proof is specified in `09-implementation-errata.md`.

Do not use full-history forks for these custom-role subagents. Full-history
forking can constrain role/model overrides and also gives the child too much
mission context. Use explicit task packets instead.

## False Positives

False positives should be survivable without a new command.

Escape routes:

1. `$interrupt` maps to `codex1 loop pause`; paused loop allows stop.
2. Invalid/ambiguous state allows stop.
3. `stop_hook_active == true` allows the second stop attempt in the same turn.

Do not add a separate `ralph override` command for v1. It is extra machinery
when pause plus the one-block rule already give a clean escape hatch.

## Implementation Sketch

```text
read stdin as StopHookInput
if parse fails:
    allow

if input.stop_hook_active:
    allow

mission = resolve_mission_from_cwd_or_active_pointer(input.cwd)
if mission cannot be resolved exactly:
    allow

status = run_status_projection(mission)
if status read/parse/validate fails:
    allow

stop = status.stop
if stop.allow:
    allow

if stop.message is empty:
    allow

if next_action is not owner=codex, required=true, autonomous=true:
    allow

if next_action.kind not in block allowlist:
    allow

emit {"decision":"block","reason":stop.message}
```

The status projection should be shared code with `codex1 status --json`.
`codex1 ralph stop-hook` should not have its own mission interpretation logic.
