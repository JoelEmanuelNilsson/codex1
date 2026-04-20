# 05 Build Prompt

Paste this prompt into a new implementation agent when you want it to build the Codex1 rebuild from this handoff folder.

```text
You are implementing Codex1 from scratch or as a clean rebuild. Treat the folder `docs/codex1-rebuild-handoff/` as the product contract. Read these files in order:

1. docs/codex1-rebuild-handoff/README.md
2. docs/codex1-rebuild-handoff/01-product-flow.md
3. docs/codex1-rebuild-handoff/02-cli-contract.md
4. docs/codex1-rebuild-handoff/03-planning-artifacts.md
5. docs/codex1-rebuild-handoff/04-roles-models-prompts.md

Also read the official OpenAI agent-friendly CLI guide:
https://developers.openai.com/codex/use-cases/agent-friendly-clis

CLI Creator is installed locally at:
- /Users/joel/.agents/skills/cli-creator/SKILL.md
- /Users/joel/.claude/skills/cli-creator/SKILL.md
- /Users/joel/.codex/skills/cli-creator/SKILL.md

The high-level product is:

Codex1 is a skills-first native Codex workflow where users invoke $clarify, $plan, $execute, $review-loop, $close, or $autopilot. These skills use a small deterministic codex1 CLI. The CLI stores visible mission files, validates a full plan with a task DAG, derives execution waves, reports next actions, records task progress, records main-thread review outcomes, pauses/resumes the active loop, checks close readiness, and emits one status JSON for Ralph. Workers execute assigned tasks. Reviewers return findings only. The main thread records mission truth. Ralph only blocks active unpaused loops by reading codex1 status --json.

Non-negotiables:

- Do not build a hidden wrapper runtime around Codex.
- Do not build fake parent/subagent permission enforcement.
- Do not add caller identity checks.
- Do not add capability-token maze.
- Do not make the CLI detect whether the caller is parent, worker, reviewer, explorer, or advisor.
- Do not use .ralph as mission truth.
- Do not store waves as editable truth.
- Do not make reviewers write review records directly.
- Do not make Ralph an orchestrator.

Implementation split:

- Skills are the user-facing product.
- CLI is deterministic substrate.
- Visible files are durable mission truth.
- Subagents are normal Codex agents governed by prompts/developer instructions.
- Ralph is a tiny stop guard that reads codex1 status --json.

Before coding:

1. Propose the exact command surface.
2. Propose file schemas for OUTCOME.md, PLAN.yaml, STATE.json, EVENTS.jsonl, specs, reviews, and CLOSEOUT.md.
3. Propose the first implementation wave as a task DAG.
4. Spawn or simulate plan critique using the roles/model guidance in 04-roles-models-prompts.md if available.
5. Ask only questions that truly block implementation.

Build order:

1. Implement the minimal CLI.
2. Implement visible mission files.
3. Implement outcome check/ratify.
4. Implement plan choose-level.
5. Implement plan scaffold/check and DAG validation.
6. Implement derived waves.
7. Implement status JSON.
8. Implement task lifecycle.
9. Implement review start/packet/record/status.
10. Implement replan dirty-count logic.
11. Implement loop pause/resume/deactivate.
12. Implement close check/complete.
13. Implement Ralph hook that only calls codex1 status --json.
14. Implement skills as thin UX wrappers over the CLI.
15. Write tests proving the contract.

Required proof:

- codex1 --help is useful.
- every command supports --json.
- invalid OUTCOME.md cannot be ratified.
- plan choose-level supports product verbs light/medium/hard, may accept 1/2/3 as aliases, records requested level, and allows main-thread effective-level escalation.
- invalid DAG is rejected.
- waves are derived from depends_on.
- waves are not stored as truth.
- task next reports ready task/wave.
- worker packet and review packet are useful.
- review record is main-thread recorded and does not require reviewer writeback.
- six consecutive dirty reviews trigger replan; clean resets the consecutive count.
- $close pauses loop and Ralph allows stop.
- codex1 status and codex1 close check agree.
- mission-close review is mandatory before close complete.
- no .ralph mission truth exists.

Use the model/role matrix:

- Main thread for hard planning: gpt-5.4 xhigh.
- Coding workers: gpt-5.3-codex high.
- Code bug/correctness reviewers: gpt-5.3-codex high.
- Intent/spec/integration/mission-close reviewers: gpt-5.4 high or xhigh.
- Explorers: gpt-5.4-mini high unless architecture judgment is needed.
- Advisors/CritiqueScout: gpt-5.4 high or xhigh.

Keep the implementation small. If you are about to add authority tokens, session identity, reviewer writeback permissions, stored wave truth, or many extra artifact files, stop and re-read the handoff. The intended design is simpler.
```
