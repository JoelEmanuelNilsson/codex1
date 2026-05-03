# Codex1

Codex1 is a deterministic artifact helper for native Codex workflows.

It does not decide whether work is ready, reviewed, correct, or done. Codex remains the semantic judge. The CLI creates and moves durable files, renders built-in templates, reports artifact inventory, and records mission-local forensic event metadata.

Long-running objective tracking belongs to native Codex goals. Use Codex's `/goal` flow to create, inspect, and complete an active goal; Codex1 only stores mission artifacts that can support that work.

For execution, Codex1 plans up to a pasteable `EXECUTION_PROMPT.md`. The user keeps the explicit go moment by copying that prompt into native `/goal`; Codex1 does not auto-start execution.

## Quickstart

To activate the Codex1 bundle for the current repository:

```sh
codex1 setup install
codex1 setup status
codex1 setup disable
codex1 setup enable
codex1 setup backups list
```

`setup install` materializes repo-scoped Codex1 skill and guidance files. It writes backups before changing managed repo guidance and never installs continuation hooks, edits global activation policy, or deletes mission artifacts.

```sh
cargo run -- --mission demo init
cargo run -- --mission demo template list
cargo run -- --mission demo interview prd --answers prd.answers.json
cargo run -- --mission demo interview plan --answers plan.answers.json
cargo run -- --mission demo inspect --json
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
  EXECUTION_PROMPT.md
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
  .codex1/
    events.jsonl
    receipts/
```

`init` creates the folders only. Interviews write content when Codex has enough answers.

Codex1 also keeps `.codex1/events.jsonl` as a mission-local forensic trail of mechanical command metadata. It is usually ignored unless Codex or a human needs to debug unusual mission history. It is not status, not proof, and not mission truth.

## Answers Files

Interviews accept JSON either as a flat object or under an `answers` key:

```json
{
  "title": "Example PRD",
  "original_request": "Build the thing",
  "interpreted_destination": "A working artifact workflow",
  "success_criteria": ["PRD exists", "Tests pass"],
  "proof_expectations": ["cargo test"],
  "pr_intent": "No PR"
}
```

String sections use strings. Repeatable sections use arrays of strings.

## Research-Heavy Flow

For uncertain work, Codex can create a PRD, then a `research-plan`, one or more `research` records, and then update the plan:

```sh
codex1 --mission demo interview research-plan --answers research-plan.json
codex1 --mission demo interview research --answers research-record.json
codex1 --mission demo interview plan --answers plan.json --overwrite
```

The CLI records what Codex learned. It does not decide that research is sufficient.

## Native Goals

Codex1 does not provide continuation commands or hook adapters. Those belonged to an older custom continuation layer that duplicated native Codex behavior.

Use native Codex goals for continuation discipline:

```text
/goal Execute the mission end to end and mark complete only after evidence is audited.
```

Codex can use mission artifacts to clarify and prove the work, but the active objective, continuation, pause/resume, accounting, budget limiting, and completion discipline live in Codex itself. Codex1 does not create, mirror, or complete native goals.

When `$plan` or an equivalent planning workflow prepares execution, it writes `EXECUTION_PROMPT.md` as the text the user can paste after `/goal`. The prompt should describe the mission, artifacts to read, subplan order, worker rules, proof/review/triage expectations, closeout criteria, and prohibited actions.

Legacy missions may contain old `.codex1/LOOP.json` files from the removed continuation system. Current Codex1 ignores those files and does not migrate them. Setup does not read, write, restore, or remove them.

## Anti-Oracle Rule

Codex1 must not expose workflow truth. In particular, `inspect` is inventory-only: artifact counts plus mechanical warnings such as missing folders or malformed frontmatter. It does not emit next actions, completion claims, review pass/fail, close gates, graph waves, native goal state, or task status.
