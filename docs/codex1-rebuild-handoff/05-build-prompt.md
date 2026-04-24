# 05 Build Prompt

Paste this prompt into a new implementation agent when you want it to build the Codex1 rebuild from this handoff folder.

```text
You are implementing Codex1 from scratch or as a clean rebuild. Treat the folder `docs/codex1-rebuild-handoff/` as the product contract. Read these files in order:

1. docs/codex1-rebuild-handoff/README.md
2. docs/codex1-rebuild-handoff/00-why-and-lessons.md
3. docs/codex1-rebuild-handoff/01-product-flow.md
4. docs/codex1-rebuild-handoff/02-cli-contract.md
5. docs/codex1-rebuild-handoff/03-planning-artifacts.md
6. docs/codex1-rebuild-handoff/04-roles-models-prompts.md
7. docs/codex1-rebuild-handoff/06-ralph-stop-hook-contract.md
8. docs/codex1-rebuild-handoff/07-review-repair-replan-contract.md
9. docs/codex1-rebuild-handoff/08-state-status-and-graph-contract.md
10. docs/codex1-rebuild-handoff/09-implementation-errata.md
11. docs/codex1-rebuild-handoff/10-first-slice-skill-contracts.md

If files 00-05 disagree with files 06-09 on Ralph, review loops, model choices,
post-lock autonomy, status verdicts, revisions, or graph/wave derivation, files
06-09 win.

Also read the official OpenAI agent-friendly CLI guide:
https://developers.openai.com/codex/use-cases/agent-friendly-clis

CLI Creator is installed locally at:
- /Users/joel/.agents/skills/cli-creator/SKILL.md
- /Users/joel/.claude/skills/cli-creator/SKILL.md
- /Users/joel/.codex/skills/cli-creator/SKILL.md

The high-level product is:

Codex1 is a way to make Codex much more powerful while keeping the user
experience native to Codex. The user-facing product is skills: users invoke
$clarify, $plan, $execute, $review-loop, $interrupt, or $autopilot. These skills
use a small deterministic codex1 CLI when durable mission state, gates,
recovery, or Ralph stop pressure are useful. Codex1 is autonomous after mission
lock: ordinary ambiguity, dirty reviews, and engineering trouble should resolve
through assumptions, repair, or replan rather than repeated user questions.
Ralph is only a minimal stop guard over codex1 status --json; it must never
become an orchestrator.

The reason for this rebuild is not that contracts are bad. It is that the old system spread contracts across too many truth surfaces and made the user feel like they were debugging the machine. Keep the contracts, but center them in a small command-shaped CLI, visible files when durable memory is useful, and clear skills.

Non-negotiables:

- Do not build a hidden wrapper runtime around Codex.
- Keep skills as the UX; do not make the CLI the product surface.
- Do not build fake parent/subagent permission enforcement.
- Do not add caller identity checks.
- Do not add capability-token maze.
- Do not make the CLI detect whether the caller is parent, worker, reviewer, explorer, or advisor.
- Do not use .ralph as mission truth.
- Do not store waves as editable truth.
- Do not require DAG/graph planning for normal work.
- Do not make reviewers write review records directly.
- Do not make Ralph depend on PreToolUse/PostToolUse observation.
- Do not expose $finish or $complete as user skills.
- Do not make Ralph an orchestrator.
- Do not use needs_user, blocked_external, or validation_required as normal post-lock execution verdicts.
- Do not let review findings become work until the main thread accepts them as blocking.
- Do not use full-history forks for Codex1 custom-role subagents.

Implementation split:

- Skills are the user-facing product.
- CLI is deterministic substrate.
- Visible files are durable mission truth when durable truth is needed.
- Subagents are normal Codex agents governed by prompts/developer instructions.
- Ralph is a minimal stop guard that reads codex1 status --json.
- Custom subagent roles disable Codex hooks so only the main/root orchestrator feels Ralph.

Before coding:

1. Propose the exact command surface.
2. Propose file schemas for OUTCOME.md, PLAN.yaml, STATE.json, EVENTS.jsonl, optional specs, optional reviews, and CLOSEOUT.md.
3. Propose adaptive planning behavior: normal plan and graph plan, including when normal work should stay chat-only.
4. For graph/large/risky planning, propose the graph contract, derived wave behavior, planned review gates, repair/replan rules, and mission-close gate.
5. Spawn or simulate plan critique using the roles/model guidance in 04-roles-models-prompts.md if available.
6. Ask only questions that truly block implementation.

Foundation vertical slice:

This is implementation order for one integrated product, not a reduced product
scope. The slice proves the substrate before graph/review/replan sits on top of
it.

1. Implement codex1 --help, codex1 init, and codex1 doctor --json.
2. Implement visible mission files with schema versions, state revision, and append-only events.
3. Implement outcome check/ratify.
4. Implement one durable normal-mode mission path with at least two steps:
   normal plan scaffold/check/lock, execute all steps, proof, task finish, and
   close complete.
5. Implement status JSON with planning_mode, verdict, next_action, loop, close, and stop semantics.
6. Implement loop activate/pause/resume/deactivate for $execute, $autopilot, and $interrupt.
7. Implement Ralph as a Codex Stop hook adapter that only uses codex1 status semantics.
8. Implement minimal normal close check/complete and CLOSEOUT.md.
9. Implement foundation skill wrappers for $clarify, $plan, $execute,
   $interrupt, and minimal $autopilot according to
   10-first-slice-skill-contracts.md.
10. Verify the installed command from outside the source folder and prove the
   normal mission slice can be driven through skills.

Then continue the same product build:

1. Implement graph plan scaffold/check and graph validation.
2. Implement derived graph waves.
3. Implement review start/packet/record/status.
4. Implement review triage, accepted-blocking finding lifecycle, two-round repair budget, and autonomous replan after repair budget.
5. Implement mission-close review and close record-review.
6. Extend skills for graph/review/replan/mission-close workflows.
7. Write broader tests proving the full integrated contract.

Required proof for the integrated product:

Foundation proof must show:

- codex1 --help is useful.
- foundation skills prove the user UX: $clarify, $plan, $execute, $interrupt,
  and a minimal $autopilot path can drive the normal mission slice without the
  user touching raw CLI commands.
- $execute continues a locked normal plan through all steps, close check, and
  close complete; it is not a one-step command.
- $autopilot follows $clarify for outcome truth and does not replace clarify
  questions with assumptions.
- $autopilot does not open a PR unless PR creation is part of the ratified
  outcome.
- codex1 doctor --json proves fast install-time assumptions without writing
  mission state; codex1 doctor --json --e2e covers deeper subagent/hook probes.
- every command supports --json.
- invalid OUTCOME.md cannot be ratified.
- plan choose-mode supports normal/graph and records requested/effective mode.
- plan choose-level supports product verbs light/medium/hard, may accept 1/2/3 as aliases, records requested/effective level, and allows main-thread escalation.
- plan lock is the only command that transitions a durable plan from valid draft to executable locked plan.
- loop activate sets the active unpaused durable loop and updates PLANS/ACTIVE.json before Ralph is expected to block.
- loop mode `execute` means continuous locked-plan execution through close
  complete; loop mode `autopilot` means full clarify/plan/execute/close
  lifecycle.
- normal plans do not require depends_on, graph waves, or planned review tasks.

Integrated graph/review/replan proof must also show:

- invalid graph plans are rejected.
- graph waves are derived from depends_on.
- waves are not stored as truth.
- task next reports ready normal step, graph task, or graph wave.
- worker packet and review packet are useful.
- review record is main-thread recorded and does not require reviewer writeback.
- review findings accept official Codex-style confidence_score and overall_confidence_score fields.
- raw review findings do not become work until triaged.
- only accepted blocking findings can block progress.
- repair is required only for current accepted blockers within repair budget.
- review repair-record increments repair_round exactly once per accepted-blocker
  repair batch.
- still dirty after repair budget triggers autonomous replan.
- $interrupt pauses loop and Ralph allows stop.
- codex1 ralph stop-hook emits valid Codex Stop-hook JSON.
- codex1 ralph stop-hook allows stop when Stop-hook input has stop_hook_active=true.
- Ralph is configurable through inline config.toml hooks.Stop and managed requirements.toml hooks.
- exact Ralph hook snippets parse through current Codex config types.
- Ralph fail-opens for missing mission, no active mission, paused, invalid-state, corrupt state, unknown next action, status error, schema mismatch, and stop_hook_active=true.
- subagent role configs disable Codex hooks with [features] codex_hooks=false.
- codex1 doctor --json --e2e or an equivalent e2e test proves a custom subagent role with codex_hooks=false does not run Ralph.
- PreToolUse/PostToolUse visibility for MCP tools, apply_patch, and long-running Bash sessions is not required for Ralph correctness.
- codex1 status and codex1 close check agree.
- codex1 close check verifies pre-close readiness; codex1 close complete writes or verifies CLOSEOUT.md and then records terminal state.
- terminal close next_action is `close_complete`; `close record-review` is reserved for mission-close review results.
- mission-close review is mandatory before close complete for graph/large/risky missions.
- no .ralph mission truth exists.

Use the model/role matrix:

The model policy is deployment-specific: use `gpt-5.5` and `gpt-5.4-mini` as
specified, and do not add runtime model-availability checks or fallback model
logic.
`gpt-5.5` is real, available in the target Codex environment, and is the latest
best model for serious Codex1 work.

- Main thread for graph planning: gpt-5.5 xhigh.
- Main thread for normal planning: gpt-5.5 high.
- Coding workers: gpt-5.5 high.
- Code bug/correctness reviewers: gpt-5.5 high.
- Intent/spec/integration/mission-close reviewers: gpt-5.5 high or xhigh.
- Explorer: gpt-5.4-mini high unless architecture judgment is needed, then gpt-5.5.
- Small mechanical workers: gpt-5.4-mini high.
- Advisors/CritiqueScout: gpt-5.5 high or xhigh.

Keep the implementation small. If you are about to add authority tokens, session identity, reviewer writeback permissions, stored wave truth, universal graph planning, or many extra artifact files, stop and re-read the handoff. The intended design is adaptive and simpler.
```
