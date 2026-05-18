# Codex1 Lane Skills Alignment PRD

## Problem Statement

Codex1 is becoming the local-first mission workflow for Codex work, but its current setup bundle only installs the core mission flow: `$clarify`, `$create-prd`, `$plan`, and `$codex1`. The execution disciplines that should happen inside a mission, such as TDD, diagnosis, architecture improvement, and prototyping, still live outside the Codex1 bundle as separate global skills.

That makes the workflow feel split. A Codex1 plan can say that a slice should use TDD or diagnosis, but the repo-local workflow does not fully carry those instructions. If global skills are removed, renamed, or changed, a repo with Codex1 installed may lose the execution guidance that the mission plan expects.

The user wants Codex1 to become the default local mission operating system while staying light, adaptable, and not overly opinionated. Codex1 should absorb the useful Matt Pocock-style workflow without losing its core behavior, and should avoid turning every mission into a bulky ceremony.

## Solution

Extend Codex1 setup so each enabled repo gets a small set of repo-local Codex1 lane skills in addition to the core mission skills.

The core skills remain:

1. `$codex1`
2. `$clarify`
3. `$create-prd`
4. `$plan`

The new Codex1 lane skills are:

1. `$tdd`
2. `$diagnose`
3. `$improve-codebase-architecture`
4. `$prototype`

Each lane skill should be copied as close to word-for-word from the existing original skill as possible. Codex1 should add only a tiny local wrapper where needed, so the skill understands mission artifacts, ready subplans, proof recording, and the native `/goal` boundary.

Codex1 should not create one mega skill. `$codex1` remains a thin overview/router. The separate skills keep context small and trigger only when relevant.

Codex1 should not revive or depend on removed unrelated skills such as `linear`, `builderjsonraw-replica`, `spec-replica-orchestrator`, or the old standalone `dogfood` skill. Instead, Codex1 should include lightweight proof/QA guidance for mission closeout, using tests, Browser, logs, screenshots, manual checks, and proof artifacts where appropriate.

## User Stories

1. As Joel, I want Codex1 setup to install the execution skills Codex1 plans rely on, so that each repo carries its own local workflow.
2. As Joel, I want Codex1 TDD to preserve the original red-green-refactor discipline, so that behavior-changing code is still driven through public-interface tests.
3. As Joel, I want Codex1 TDD to understand mission subplans and acceptance criteria, so that implementation slices can turn plan expectations into tests.
4. As Joel, I want Codex1 TDD to avoid fake ceremony for docs, mechanical edits, and prototypes, so that the workflow stays practical.
5. As Joel, I want Codex1 Diagnose to preserve the original reproduce-first diagnosis loop, so that hard bugs are debugged through deterministic feedback signals.
6. As Joel, I want Codex1 Diagnose to record repro loops, hypotheses, fixes, and regression proof in mission artifacts, so that bug work leaves durable evidence.
7. As Joel, I want Codex1 Improve Codebase Architecture to preserve the deep-module language, so that architecture work keeps using modules, interfaces, seams, adapters, depth, leverage, and locality.
8. As Joel, I want Codex1 Improve Codebase Architecture to support both feature-mission architecture lanes and architecture-only missions, so that refactoring can be either supporting work or the main mission.
9. As Joel, I want Codex1 Prototype to preserve the original throwaway-prototype discipline, so that prototypes answer named questions and do not become accidental production code.
10. As Joel, I want Codex1 Prototype to work before PRD, during planning, or during execution, so that uncertainty can be resolved at the right point.
11. As Joel, I want `$plan` to assign a simple execution lane to every ready subplan, so that `/goal` has clear guidance without needing a heavy dependency graph.
12. As Joel, I want the execution lane field to include `standard`, so that simple docs, setup, config, and mechanical work do not need fake TDD or diagnosis.
13. As Joel, I want Codex1 proof/QA to verify mission acceptance criteria, so that closeout proves the mission instead of roaming the whole app for unrelated bugs.
14. As Joel, I want Browser-based QA to be available for web UI proof, so that Codex can inspect the actual app when acceptance criteria involve UI behavior.
15. As Joel, I want the old standalone dogfood workflow removed from the expected Codex1 flow, so that broad exploratory QA does not become mandatory mission ceremony.
16. As Joel, I want setup to remain repo-local, so that Codex1 does not edit or depend on global skills.
17. As Joel, I want setup install/update to be non-destructive, so that running setup frequently is safe.
18. As Joel, I want explicit prune or uninstall behavior for deletion, so that removed managed files are not surprising.
19. As Joel, I want `agents/openai.yaml` metadata installed when practical, so that Codex1 skills are nicer in the UI without making metadata the source of truth.
20. As Joel, I want `GOAL_BRIEF.md` to remain the bridge into native `/goal`, so that Codex1 does not become a goal engine.
21. As a future Codex agent, I want each repo-local skill to include enough local mission context, so that I can execute a mission even if global skills are absent.
22. As a future Codex agent, I want the original skill behavior preserved, so that the Codex1 versions do not drift into vague or weaker copies.
23. As a maintainer, I want tests proving setup installs the expanded bundle, so that the lane skills do not silently disappear.
24. As a maintainer, I want stale setup bundles to report accurately, so that repos can be refreshed when the managed skill set changes.
25. As a maintainer, I want the docs to explain the relationship between Codex1 core skills and lane skills, so that users do not have to remember the whole conversation.

## Implementation Decisions

- Codex1 setup should install repo-local lane skills in `.agents/skills/` for `tdd`, `diagnose`, `improve-codebase-architecture`, and `prototype`.
- The lane skills should be full local files, not references to global skills. Local-first repos should still work if global skills are removed.
- Each lane skill should preserve the original skill body as closely as possible. Codex1-specific changes should be small, explicit wrappers or minimal local edits.
- `$tdd` should keep the red-green-refactor workflow, public-interface testing principle, anti-horizontal-slice warning, and refactor-after-green rule.
- `$diagnose` should keep the reproduce-first loop: feedback loop, reproduce, hypotheses, instrumentation, fix plus regression test, cleanup, and post-mortem.
- `$improve-codebase-architecture` should keep the deep-module vocabulary and deepening-opportunity flow.
- `$prototype` should keep the throwaway prototype rules and its two branches: logic/state prototypes and UI prototypes.
- `$plan` should add a required `Execution Lane` field to every ready subplan.
- Allowed execution lanes should be `tdd`, `diagnose`, `improve-codebase-architecture`, `prototype`, `proof-qa`, and `standard`.
- `standard` should be the escape hatch for docs, simple config, mechanical updates, low-risk chores, and other work where a specialist lane would be artificial.
- `$plan` should assign lanes but not execute them. Native `/goal` execution uses the lane guidance when it reaches each ready subplan.
- Architecture work should be allowed both as a lane inside a feature mission and as the main purpose of an architecture refactor mission.
- Prototype work should be allowed before PRD, during planning, or during `/goal` execution, but every prototype must answer a named question and produce a durable answer.
- Repo-wide domain language and durable architecture decisions should remain in `CONTEXT.md` and `docs/adr/`.
- Mission-specific decisions, proof, triage, and closeout should remain in `.codex1/missions/<mission-id>/`.
- Codex1 should not depend on `linear`, `builderjsonraw-replica`, `spec-replica-orchestrator`, or old standalone `dogfood`.
- The old dogfood concept should be replaced in Codex1 by lightweight mission proof/QA guidance, not a broad exploratory QA requirement.
- `plan-codex-goal` and `write-codex-goal` should be left alone as optional global helper skills. Codex1 should not depend on them.
- Setup should remain repo-local and should not edit `/Users/joel/.agents/skills` or `/Users/joel/.codex/skills`.
- Setup install/update should add or update managed Codex1 files but should not delete unmanaged files.
- Any deletion of managed-but-retired files should require explicit prune or uninstall behavior.
- `SKILL.md` remains the source of truth for each skill. `agents/openai.yaml` is UI metadata only.

## Testing Decisions

- Setup bundle tests should verify that `codex1 setup install` materializes all core skills and all lane skills.
- Setup status tests should verify that stale or missing lane skills are reported accurately.
- Setup uninstall tests should verify that managed lane skills are removed only by explicit uninstall behavior.
- Setup should continue to reject unsafe writes through symlinks or paths escaping the repo.
- Tests should verify that setup remains repo-local and does not touch global skill directories.
- Tests should verify that the managed bundle marker includes the expanded managed file set and bundle version.
- Template or skill-content tests should verify that ready subplan guidance includes a required `Execution Lane` field and lists the allowed lanes.
- Docs tests or text assertions should verify that Codex1 workflow documentation explains core skills, lane skills, native `/goal`, and lightweight proof/QA.
- Existing CLI tests should continue to pass, including init, interview, inspect, backup, setup status, setup backup restore, and anti-oracle behavior.
- Good tests should verify observable behavior through CLI output, created files, setup status JSON, and installed file contents rather than private helper implementation.

## Out of Scope

- Do not implement a new full dogfood replacement skill in this mission.
- Do not install or remove global skills automatically.
- Do not integrate Linear, GitHub Issues, Jira, or any other issue tracker.
- Do not create a mega skill that loads every Codex1 instruction at once.
- Do not make TDD mandatory for docs-only, mechanical, prototype, or no-meaningful-seam work.
- Do not make Browser QA a broad exploratory app audit by default.
- Do not change native `/goal` behavior or try to create, inspect, mirror, or complete native goals from Codex1.
- Do not make `agents/openai.yaml` semantically authoritative.
- Do not require every mission to run every lane.

## Further Notes

- The main user-facing model should remain short: `$clarify`, `$create-prd`, `$plan`, then create a native `/goal` from the Codex1 mission.
- Advanced usage should be optional: use `$prototype` for uncertainty, `$diagnose` for hard bugs, `$improve-codebase-architecture` for architecture work, and `$tdd` for behavior-changing code slices.
- Proof/QA should be scoped to mission acceptance criteria. It should prove the mission, not wander the entire app looking for unrelated issues.
- Codex1 should stay adaptable. The plan should guide Codex execution without turning into a project-management machine.
- The expanded setup bundle should make future repos self-contained enough that Codex can understand and execute Codex1 missions without relying on the user's old global skill set.

