# Codex1

Codex1 is a deterministic artifact helper for native Codex workflows.

It does not decide whether work is ready, reviewed, correct, or done. Codex remains the semantic judge. The CLI creates and moves durable files, renders built-in templates, reports artifact inventory, and manages a tiny explicit continuation loop for Ralph.

## Quickstart

To activate the Codex1 bundle for the current repository:

```sh
codex1 setup install
codex1 setup status
codex1 setup disable
codex1 setup enable
```

`setup install` installs the global Ralph hook capability and enables only the current repo by default. It writes backups before config mutations and materializes Codex1-managed repo skills/guidance only where Codex1 is enabled.

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
    LOOP.json
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

## Loop And Ralph

The explicit loop is opt-in:

```sh
codex1 --mission demo loop start --mode autopilot --message "Continue the mission until the current slice is handled."
codex1 --mission demo loop pause --reason "User interrupted"
codex1 --mission demo loop resume
codex1 --mission demo loop stop --reason "Mission closed"
```

`codex1 ralph stop-hook` reads Stop-hook JSON from stdin. It blocks only when the mission has active, unpaused loop state with a non-empty message. Missing, corrupt, inactive, paused, or recursive hook input allows stop.

When setup policy exists, global Ralph hooks check repo activation before scanning loop state. Disabled repos, malformed setup policy, and unresolved repos fail open. Project-local hooks use `ralph stop-hook --scope project` so migration to project setup keeps that repo's hook effective without re-enabling the global hook.

## Anti-Oracle Rule

Codex1 must not expose workflow truth. In particular, `inspect` is inventory-only: artifact counts plus mechanical warnings such as missing folders or malformed frontmatter. It does not emit next actions, completion claims, review pass/fail, close gates, graph waves, or task status.
