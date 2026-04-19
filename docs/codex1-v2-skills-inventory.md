# Codex1 V2 Skills Inventory

Repo-local skills live at `.claude/skills/<name>/SKILL.md` (Claude Code)
and `.codex/skills/<name>/SKILL.md` (Codex). The two trees are byte-identical
mirrors, maintained that way since the T44 cutover.

User-global skill directories (`~/.claude/skills/`, `~/.codex/skills/`)
are **out of scope** for V2. If you want global availability, symlink
the repo-local files yourself.

Every skill begins with a resolver preamble that sets `$CODEX1` via
`${CODEX1_REPO_ROOT:-/Users/joel/codex1}/scripts/resolve-codex1-bin`;
the CLI-command column below abbreviates `"$CODEX1"` as `codex1` for
readability, but the skill files themselves always invoke `"$CODEX1"`.

## Skills

| Skill | Purpose | When | Drives CLI |
| --- | --- | --- | --- |
| `$clarify` | Ratify `OUTCOME-LOCK.md` | New mission, or lock_status: draft | `codex1 init`, `codex1 validate` |
| `$plan` | Author the PROGRAM-BLUEPRINT DAG | Lock ratified but waves empty; replans | `codex1 plan check`, `plan graph`, `replan record` |
| `$execute` | Run a single task to `proof_submitted` | task next emits start_task | `codex1 parent-loop activate --mode execute`, `task start`, `task finish` |
| `$review-loop` | Open review, process findings, route | task in `proof_submitted` or `review_owed` | `codex1 parent-loop activate --mode review`, `review open/submit/status/close`, `replan check` |
| `$close` | Pause the active parent loop | User invokes $close, mid-mission discussion | `codex1 parent-loop pause`, `parent-loop resume` |
| `$autopilot` | Compose all of the above end-to-end | User wants hands-off progression | All of the above + `review open-mission-close`, `mission-close check/complete` |

## Composition map

```
                   $autopilot
                       │
       ┌───────────────┼────────────────┐
       │               │                │
   $clarify         $plan         $execute ◀─► $review-loop
                                     │               │
                                     └───── $close ──┘ (pause)
                                     │
                                     └──► codex1 review open-mission-close
                                                   │
                                          codex1 mission-close check/complete
```

## Authority model

Each skill respects the retrospective's authority boundaries:

- **Parent** (skill invoker): owns mission truth, writeback decisions,
  repair/replan routing, mission completion.
- **Reviewer** (subagent spawned by `$review-loop`): owns bounded
  findings or `NONE`. Cannot clear gates or terminalize.
- **Advisor** (optional checkpoint invokee): owns strategic advice.
  **Not formal review evidence.**
- **Worker** (subagent spawned by `$execute`): owns bounded
  implementation + proof. Cannot mutate mission truth.
- **Ralph**: owns stop/resume guard over
  `codex1 status --mission <id> --json`. Invoked by
  `scripts/ralph-status-hook.sh` in scan mode by default — the hook
  walks up from `$PWD` to the nearest `PLANS/` directory, iterates every
  mission under it, and blocks stop if any reports
  `stop_policy.allow_stop: false`. Ralph does no planning, executing,
  reviewing, or subagent spawning.
- **User**: owns interruption (`$close`) and direction.

## Parent loop modes

Every skill that runs CLI mutations activates the parent loop first:

| Skill | Mode |
| --- | --- |
| `$execute` | `execute` |
| `$review-loop` | `review` |
| `$autopilot` | `autopilot` |
| `$close` | (pauses whichever mode is active) |
| `$clarify`, `$plan` | (no parent loop — interactive editing) |

## Qualification

V2's "done" bar has two halves:

- **Repo-ready** — the code, verifier, hook, skills, and tests shipped
  in this commit satisfy every static gate (fmt/clippy/test green,
  hook scans correctly, verifier rejects fabricated receipts). This
  half is what the checked-in repo asserts.
- **Operator-qualified** — a live `$autopilot` run produces
  `docs/qualification/codex1-v2-e2e-receipt.md`; `scripts/qualify-codex1-v2.sh
  verify` accepts it. This half is operator-owned by design — see
  `docs/codex1-v2-operator-guide.md#qualification`. The repo cannot
  produce this receipt itself.
