---
name: create-prd
description: Synthesize known context into a Codex1 PRD artifact. Use when the user wants a PRD from the current conversation, clarification output, repo context, and references; do not publish to an issue tracker.
---

# Create PRD

Use this after `$clarify` or whenever the user asks for a PRD from known context. This is the local Codex1 version of the reference `to-prd` workflow: same synthesis quality, no issue tracker.

Do not interview the user by default. Synthesize what is already known. If important information is missing, record an assumption, risk, or open question in the PRD instead of blocking. Only ask when the user explicitly wants to co-author, when two authoritative sources contradict each other, or when the module/test choices are load-bearing and the user is clearly still in the loop.

## Process

1. Explore the repo enough to understand the current state before writing. Read the conversation, clarification brief, existing mission artifacts, `AGENTS.md`, `docs/agents/codex1-workflow.md`, `docs/agents/codex1-domain.md`, `docs/agents/codex1-artifact-briefs.md`, `CONTEXT.md` or `CONTEXT-MAP.md`, ADRs, tests, and relevant source.
2. Write from the user's perspective first: problem, solution, actors, user stories, and externally visible outcomes.
3. Use the codebase's own language. Prefer terms from docs, source, tests, and ADRs over invented vocabulary.
4. Sketch the major modules or areas likely to be built or modified. Actively look for deep modules: simple interfaces that hide meaningful complexity and can be tested in isolation.
5. If the module sketch or test focus is uncertain and important, briefly check with the user exactly like the reference PRD skill. If the user is not available or did not ask for interaction, proceed and record assumptions.
6. Capture implementation decisions: module boundaries, interface changes, architectural decisions, schema changes, API contracts, integrations, state ownership, and specific interactions.
7. Capture testing decisions: what external behavior proves success, which modules deserve direct tests, prior art in the existing test suite, and what not to test because it is implementation detail.
8. Write `PRD.md` locally through the Codex1 PRD artifact workflow. Do not publish it anywhere.

## PRD Shape

The PRD must be good enough that `$plan` can design execution without reconstructing the product intent. Include:

- Problem Statement: the problem from the user's perspective.
- Solution: the solution from the user's perspective.
- User Stories: a long numbered list covering the whole feature, in "As an <actor>, I want <feature>, so that <benefit>" form.
- Success Criteria: observable facts that make the PRD satisfied.
- Module Sketch: likely modules, interfaces, and deep-module opportunities, using stable names rather than brittle paths.
- Implementation Decisions: modules, interfaces, architecture, schemas, API contracts, state boundaries, integrations, and clarified interactions.
- Testing Decisions: external behavior to test, modules to test directly, prior test patterns, and testing non-goals.
- Out of Scope: what this PRD intentionally does not include.
- Constraints, verified context, assumptions, resolved questions, proof expectations, review expectations, and PR intent.
- Further Notes when useful.

Do not include code snippets that will go stale quickly, and do not include brittle file paths; brittle paths make the PRD age badly. It is fine to mention stable module names, artifact names, commands, and durable concepts.

Do not publish to GitHub Issues, Linear, Jira, or any issue tracker. Codex1 PRDs stay in the mission artifact tree. Do not start implementation, create `PLAN.md`, or create/complete native `/goal` state.
