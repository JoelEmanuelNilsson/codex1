# Codex1

Codex1 is a tiny setup and mission-scaffold helper for native Codex workflows.

It does not decide whether work is ready, reviewed, correct, or done. Codex remains the semantic judge. The CLI only materializes repo-local Codex1 guidance and creates a path-safe mission directory layout.

Long-running objective tracking belongs to native Codex goals. Use Codex's `/goal` flow to manage the active goal; Codex1 only stores mission artifacts that can support that work.

Codex1 skills clarify product intent and synthesize PRDs; execution stays with Codex directly or with a native `/goal` when the user wants persistence. The CLI does not auto-start execution or generate semantic mission artifacts.

## Quickstart

To activate the Codex1 bundle for the current repository:

```sh
codex1 setup
codex1 setup install
codex1 setup status
codex1 setup disable
codex1 setup enable
codex1 setup backups list
```

`codex1 setup` is the short install/repair command. `setup install` is the explicit form. Both materialize repo-scoped Codex1 skills and guidance files, write backups before changing managed repo guidance, and never delete mission artifacts.

The installed skills expose the intended UX:

```text
$clarify     -> gather mission context while questions are allowed
$create-prd  -> synthesize known context into PRD.md
$codex-review -> run advisory Codex review before proof/closeout when useful
$handoff     -> compact the session into a temporary note for a fresh agent
/goal        -> user creates or refines the native goal when persistence is useful
```

```sh
cargo run -- --mission demo init
cargo run -- --json --mission demo init
cargo run -- --json setup status
```

Mission artifacts live under:

```text
.codex1/missions/<mission-id>/
```

The mission ID is intentionally boring: ASCII letters, digits, `-`, and `_` only.

## Artifact Tree

```text
.codex1/missions/<mission-id>/
  PRD.md
  PLAN.md
  RESEARCH_PLAN.md
  GOAL_BRIEF.md
  GOAL_PROMPT.md
  CLOSEOUT.md
  RESEARCH/
  SPECS/
  SUBPLANS/
    ready/
    active/
    done/
    paused/
    superseded/
  ADRS/
  REVIEWS/
  TRIAGE/
  PROOFS/
```

`init` creates the folders only. The workflow skills create mission content when Codex has enough context. `GOAL_PROMPT.md` is optional and should appear only when a separate pasteable native-goal prompt is useful.

## Research-Heavy Flow

For uncertain work, Codex can create a PRD, then a research plan and one or more research records as normal Markdown files inside the mission tree. The CLI does not decide that research is sufficient.

## Native Goals

Codex1 does not provide native-goal commands.

Use native Codex goals for continuation discipline:

```text
/goal Execute the mission end to end and mark complete only after evidence is audited.
```

Codex can use mission artifacts to clarify and prove the work, but the active objective, continuation, pause/resume, accounting, budget limiting, and completion discipline live in Codex itself. Codex1 does not create, mirror, or complete native goals.

When persistent execution is useful, Codex can use the PRD, current repo evidence, and any existing mission artifacts to create or refine a whole-mission `/goal`. If the user needs exact copy/paste text under a limit, keep that as a compact `GOAL_PROMPT.md` rather than turning the CLI into a goal author.

## Anti-Oracle Rule

Codex1 must not expose workflow truth. There is no mission-status command: artifact quality, readiness, proof sufficiency, review cleanliness, close safety, and native goal state remain Codex judgments made from the actual files.
