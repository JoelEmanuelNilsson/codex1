# Claude Code Build Prompt For Codex1 V2

You are Claude Code building Codex1 V2 in `/Users/joel/codex1`.

Important: do not build V2 by extending or trusting the current Codex1
mission/Ralph runtime machinery. The current repo is useful reference material,
but the V1 runtime became too overengineered and too stateful. Treat the V2 docs
as the source of truth.

Read these first:

- `/Users/joel/codex1/docs/codex1-v2-prd.md`
- `/Users/joel/codex1/docs/codex1-v2-architecture-brief.md`
- `/Users/joel/codex1/docs/codex1-v2-cli-contract.md`
- `/Users/joel/codex1/docs/codex1-run-retrospective-2026-04-18.md`
- OpenAI CLI guide: https://developers.openai.com/codex/use-cases/agent-friendly-clis
- Installed cli-creator paths:
  - `/Users/joel/.claude/skills/cli-creator/SKILL.md`
  - `/Users/joel/.codex/skills/cli-creator/SKILL.md`

## North Star

Build Codex1 V2 as:

```text
skills-first UX + small CLI contract kernel + tiny Ralph status guard +
normal bounded subagents + visible files
```

Do not recreate the old giant runtime.

## Product Scope

This is not a tiny MVP. The target is the full Codex1 workflow:

```text
clarify -> plan DAG -> execute waves -> proof -> review-loop -> repair/replan
-> pause/resume -> autopilot -> mission close
```

But build it in dependency waves:

1. Kernel and DAG.
2. Task execution and proof.
3. Review and repair/replan.
4. Ralph and skill composition.
5. Autopilot, advisor checkpoints, mission close, end-to-end qualification.

## Hard Constraints

- Public skills remain the user-facing product.
- The CLI is the deterministic backend.
- Ralph only calls `codex1 status --mission <id> --json`.
- Subagents are normal bounded workers/reviewers and are never in Ralph.
- Every plan must include explicit task IDs and `depends_on`.
- Waves are derived from the DAG and state; they are not canonical truth.
- `STATE.json` owns mutable operational state.
- `PROGRAM-BLUEPRINT.md` owns immutable route/DAG truth.
- Task IDs must never be reused within a mission.
- Reviewer outputs must bind to graph/state/evidence revisions.
- Late stale outputs must be rejected or quarantined.
- Parent self-review must not count as review evidence.
- Mission completion requires mission-close review.

## Build Guidance

Start by designing the task DAG for the implementation itself. Do not write
major code until the plan has:

- tasks `T1`, `T2`, etc.
- explicit `depends_on`
- read/write paths
- exclusive resources
- proof rows
- review profiles
- derived build waves

Use the installed `cli-creator` skill if useful:

- Codex: `/Users/joel/.codex/skills/cli-creator/SKILL.md`
- Claude: `/Users/joel/.claude/skills/cli-creator/SKILL.md`
- Agents: `/Users/joel/.agents/skills/cli-creator/SKILL.md`

## First Implementation Wave

Implement the kernel and DAG commands first:

```bash
codex1 init --mission <id> --title <title> --json
codex1 validate --mission <id> --json
codex1 status --mission <id> --json
codex1 plan check --mission <id> --json
codex1 plan waves --mission <id> --json
codex1 task next --mission <id> --json
```

This first wave must prove:

- mission files can be created
- `OUTCOME-LOCK.md` can be validated structurally
- `PROGRAM-BLUEPRINT.md` task DAG can be parsed from strict markers
- malformed DAGs are rejected
- ready tasks and waves are derived
- status JSON is stable and deterministic
- status includes `schema`, `state_revision`, `verdict`, `parent_loop`,
  `stop_policy`, and typed `next_action`

## Do Not

- Do not start with autopilot.
- Do not start with review writeback machinery.
- Do not add a database.
- Do not add a daemon.
- Do not make Ralph smart.
- Do not make skills duplicate CLI validation logic.
- Do not keep multiple canonical state surfaces.
- Do not use the old runtime as an architectural template.

## Definition Of Done For V2

V2 is done when a user can:

1. Clarify a mission.
2. Plan with `light`, `medium`, or `hard`.
3. Get a valid dependency DAG.
4. Execute dependency-safe waves.
5. Run proof.
6. Run review through durable reviewer outputs.
7. Repair or replan honestly.
8. Pause and resume via `$close`.
9. Use `$autopilot` for the full flow.
10. Reach mission-close review and terminal completion without hidden state or
    stale subagent contamination.

Be ruthless about simplicity. If a feature does not strengthen clarify truth,
planning DAG quality, execution safety, review honesty, Ralph continuation, or
mission close, it is probably not core V2.
