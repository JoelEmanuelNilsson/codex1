Codex1 is to be making the experience with codex as good as possible. That is partly by having skills be token-efficient, and removing "questionmarks" the llm's might have regarding stuff; the reasoning here is that if something is not clear in our skills, then the llm will use it's reasoning effort to reason on what it should do, hence making the llm dumber if it needs to think about stuff that is not clear, giving it context bloat.

<!-- codex1-managed setup guidance start -->
# Codex1 Setup Guidance

codex1-managed

Codex1 is enabled in this repository as a local artifact workflow convention. Use `$clarify`, `$create-prd`, and `$plan` for the mission workflow. Read `docs/agents/codex1-workflow.md`, `docs/agents/codex1-domain.md`, and `docs/agents/codex1-artifact-briefs.md` for the repo-local workflow, domain, ADR, and artifact rules. Use `codex1 setup` for repo-local guidance and `codex1 init` for path-safe mission scaffolding. Use native `/goal` for persistent objectives and continuation. The preferred flow is clarify, create PRD, plan, then create or refine the native goal from `GOAL_BRIEF.md`.

Codex remains the semantic judge. Codex1 setup status and init output are not readiness, completion, review, proof, closeout, or native goal state.
<!-- codex1-managed setup guidance end -->
