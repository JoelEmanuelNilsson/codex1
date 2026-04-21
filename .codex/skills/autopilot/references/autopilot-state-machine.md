# Autopilot State Machine

Full dispatch table and pseudocode for `$autopilot`.

Every iteration re-reads `codex1 status --json` and dispatches on the combination of `verdict`
and `next_action.kind`. Trust the envelope; do not cache.

## Dispatch table

| verdict                         | next_action.kind         | Autopilot action                               |
|---------------------------------|--------------------------|------------------------------------------------|
| needs_user                      | clarify                  | `$clarify`                                     |
| needs_user                      | plan                     | `$plan` (choose level, then invoke)            |
| continue_required               | run_task                 | `$execute`                                     |
| continue_required               | run_wave                 | `$execute`                                     |
| continue_required               | run_review               | `$review-loop` (planned task mode)             |
| continue_required               | repair                   | `$execute` (repair task)                       |
| blocked                         | repair                   | `$execute` (repair task)                       |
| blocked                         | replan                   | `$plan replan`                                 |
| ready_for_mission_close_review  | mission_close_review     | `$review-loop` (mission-close mode)            |
| mission_close_review_open       | mission_close_review     | `$review-loop` (mission-close mode)            |
| mission_close_review_passed     | close                    | `codex1 close complete`                        |
| terminal_complete               | closed                   | Stop; report terminal                          |
| invalid_state                   | fix_state                | Escalate to user; do not auto-fix              |

Any `verdict` + `next_action.kind` combination not in this table is a surface for user
escalation, not for silent progress.

## Pause-for-user triggers

Independent of the dispatch table, `$autopilot` MUST pause when any of the following are detected
during any step (pre-status, during sub-skill, on tool output, or on user message):

- Scope change requested or discovered.
- Risk discovery (security, data loss, architectural mismatch).
- Money / billing / API credits / deployment steps.
- Destructive operations (force-push, prod data deletion, irreversible migration).
- External systems not scoped in OUTCOME.md.
- `blocked` verdict with a non-obvious resolution.
- User invoked `$close`.

## Pseudocode main loop

```text
autopilot(mission_id):
    loop:
        if close_requested():
            yield_to_user()        # $close -> pause; do not resume without explicit agreement
            return

        status = run("codex1 --json status --mission " + mission_id)

        if status.ok is False:
            if status.retryable:
                continue           # transient (e.g. REVISION_CONFLICT) -> re-read
            escalate(status)       # non-retryable CLI error
            return

        verdict = status.data.verdict
        action  = status.data.next_action
        kind    = action.kind

        # Terminal / closed
        if verdict == "terminal_complete" or kind == "closed":
            report_terminal(status)
            return

        # Explicit close step: finish terminal close
        if kind == "close":
            if status.data.close_ready is False:
                escalate(status)   # status disagrees with itself -> user input
                return
            run("codex1 close complete --mission " + mission_id)
            continue               # next iteration should observe terminal_complete

        # Escalations we must not attempt to resolve
        if verdict == "blocked" and kind not in {"repair", "replan"}:
            escalate(status)
            return
        if verdict == "invalid_state" or kind == "fix_state":
            escalate(status)
            return

        # Pause-for-user triggers fire here even if dispatch would otherwise proceed
        if requires_genuine_user_input(status):
            escalate(status)
            return

        # Normal dispatch
        match kind:
            case "clarify":
                invoke("$clarify", mission_id)
            case "plan":
                level = choose_plan_level(status)           # medium default, escalate on risk
                run("codex1 plan choose-level --mission " + mission_id + " --level " + level)
                invoke("$plan", mission_id, level)
            case "replan":
                invoke("$plan", mission_id, mode="replan")
            case "run_task" | "run_wave":
                invoke("$execute", mission_id, action)      # one step
            case "repair":
                invoke("$execute", mission_id, action)      # repair task is still a task
            case "run_review":
                invoke("$review-loop", mission_id, mode="planned_task")
            case "mission_close_review":
                invoke("$review-loop", mission_id, mode="mission_close")
            case _:
                escalate(status)
                return

        # Loop back to Step 1: re-read status
```

## Pause-on-close handshake

`close_requested()` must be checked at the top of every iteration. The user's `$close` invocation
translates to `codex1 loop pause --json`; until `codex1 loop resume --json` runs (by explicit
user decision), `$autopilot` yields. Do not inspect user chat to infer intent to resume.

## Planning-level selection

See the "Planning level" section in `SKILL.md`. In short: `medium` default, `hard` on any
pause-for-user trigger, `light` only for explicitly small/local missions. Record via
`codex1 plan choose-level`; honor CLI-returned escalation.

## Do-not-do list

- Do not run `codex1 close complete` unless `status.close_ready == true` and the most recent
  `codex1 close check --json` returned `ready: true`.
- Do not mutate STATE.json / PLAN.yaml / OUTCOME.md directly.
- Do not record review verdicts; that is `$review-loop`'s job.
- Do not attempt to resolve `invalid_state` automatically.
- Do not resume after `$close` without explicit user agreement.
