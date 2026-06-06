You are a prep worker, not the final implementation worker.

Repository: /Users/joel/codex1
Artifact output directory: /Users/joel/codex1/docs/goals/20260606-codex1-setup-refactor

Objective to prepare:
Prepare a future Codex /goal to fundamentally refactor Codex1 setup maintenance so Codex1 is much easier to use, update, and maintain across all projects. Codex1 is Joel's universal repo setup layer: the curated skills, setup guidance, managed docs, setup bundle marker, backups, upgrade/removal behavior, and fleet update path that should work for every valid local project. It is not "just some optional skills."

Why this matters:
A recent Codex thread took about 20 minutes and 107 tool/function calls to remove three repo-local skills from Codex1 and attempt the update-codex1-setups flow. The work fanned out across setup catalog arrays, marker JSON, legacy bundle compatibility, deletion fingerprints, docs, tests, and a shell updater that refused to apply because the source repo was dirty. The desired future state is that changing, retiring, validating, publishing, and fleet-updating the Codex1 setup bundle is direct, safe, and agent-drivable instead of architecture archaeology.

Important context and constraints:
- Do not implement the final refactor in this prep run.
- Do not commit, push, or run fleet updates in this prep run.
- The current worktree may already contain uncommitted setup/skill-removal changes; preserve them and report how that affects readiness.
- Read repo guidance first: AGENTS.md, CONTEXT.md, README.md, docs/agents/*.md, docs/setup-bundle.md, docs/cli-contract.md.
- Relevant current implementation likely includes src/setup/catalog.rs, src/setup/mod.rs, src/setup/workspace.rs, src/cli.rs, tests/setup.rs, tests/common/mod.rs, and .agents/skills/update-codex1-setups/scripts/update-codex1-setups.sh.
- Use Codex1 domain language where appropriate: Codex1 CLI, Setup bundle, Mission scaffold, Native goal, Local-first mission, Evidence artifact, Anti-Oracle Rule.
- The future goal should not weaken safety: do not discard user changes, do not make setup overwrite user-authored files, do not push unrelated commits, and do not turn Codex1 into a native goal/workflow-state oracle.

Do not launch another nested Codex worker.

Read the repo and determine:
1. What exactly should be refactored and why.
2. Current baseline: concrete friction points, shallow modules, duplicated bundle sources, manual update steps, tests that mirror implementation, updater limitations, and any dirty-worktree risks.
3. Success metrics/targets for "much easier to use, update, maintain" that a future goal can prove objectively.
4. Ranked implementation strategy with expected impact, risk, validation method, and rejected alternatives.
5. Validation loop and anti-gaming constraints.
6. Execution-readiness audit for only this mission: source control, local runtime/toolchain, tests, fleet updater behavior, commits/pushes, external services, cost/quota, safety, and production/data risk.
7. Relevant skills/plugins/docs the future execution agent should use.

Create/update these files in the output directory:
- PREP.md
- READINESS_AUDIT.md
- GOAL.md, only if the future goal is unblocked
- BLOCKERS.md, only if required blockers remain

If required blockers remain, do not create a final executable GOAL.md. If unblocked, GOAL.md must contain exactly one copy/pasteable /goal command under 4000 characters, with exactly these sections: Objective, Context, Success criteria, Feedback loop, Tracking, Constraints, Completion report.

The final /goal must be reducible to:
"/goal <desired end state> verified by <specific evidence> while preserving <constraints>. Between iterations, <how Codex chooses the next best action>."

The Tracking section must require at least one notes.md file for running notes, decisions, measurements, blockers, and next steps. The Completion report must require evidence, commands run, changed artifacts, residual risks, and cleanup/review performed. Do not include a generic stop/ask section.

Final response:
- Files created
- Mission logic
- Metrics/targets
- Key evidence and commands run
- Readiness status and blockers/assumptions
- Final /goal character count
- Final /goal, when one exists
