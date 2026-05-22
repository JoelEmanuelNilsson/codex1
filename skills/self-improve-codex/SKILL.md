---
name: self-improve-codex
description: "Codex-only session introspection for evidence-backed skill, AGENTS.md, and memory improvement proposals."
---

# Self Improve Codex

## Purpose

Use this Codex-only power skill to inspect local Codex session evidence, find repeated improvement signals, and propose durable changes to Codex skills, project or global `AGENTS.md`, and workspace memory. It combines Codex-native self-improvement with technical skill discovery.

This is not a Cursor, Claude, or general agent-log workflow. Assume Codex local state, Codex rollout JSONL files, Codex skill folders, Codex project/global `AGENTS.md` conventions, and, when the current repo uses it, Codex1 artifact conventions.

Default stance: propose first. Do not patch `SKILL.md`, project `AGENTS.md`, global `~/.codex/AGENTS.md`, `MEMORY.md`, daily memory, or other instruction files unless the user explicitly approves the exact target and change.

## Source Discovery

Prefer Codex's authoritative local session index:

1. `~/.codex/state_5.sqlite`
2. `threads.rollout_path` values from that SQLite index
3. rollout files under `~/.codex/sessions/**/*.jsonl` and `~/.codex/archived_sessions/**/*.jsonl`

Gracefully fall back to supporting sources when the database is missing, locked, or its schema changed:

- `~/.codex/sessions/**/*.jsonl`
- `~/.codex/archived_sessions/**/*.jsonl`
- `~/.codex/history.jsonl`
- `~/.codex/session_index.jsonl`
- `~/.codex/log/**/*.jsonl`
- repo-local telemetry or instructions such as `AGENTS.md`, `docs/agents/*`, and existing skill metadata

Treat incomplete convenience indexes such as `session_index.jsonl` as supporting evidence only. They can help find files, but they are not the source of truth when `state_5.sqlite` is usable.

When schema assumptions fail, inspect SQLite tables and columns, report the mismatch, and degrade to path inventory. Do not fail the whole workflow just because one Codex version moved a column.

Use the bundled safe inventory helper when useful:

```bash
python3 skills/self-improve-codex/scripts/scan_codex_sessions.py --recent 20
```

The helper only counts JSONL files and prints session metadata or rollout paths. It does not parse or dump full transcripts.

## Operating Modes

### `scan`

Inventory available Codex session sources. Report whether `~/.codex/state_5.sqlite` exists, whether a `threads` table is readable, which rollout/session directories exist, JSONL counts, and recent rollout paths. Use this before deeper analysis.

### `dream`

Run a broad introspection pass over recent sessions. Mine repeated user corrections, preference nudges, frustration cues, autonomy/persistence gaps, tool-use mistakes, and instruction gaps. Output evidence-backed proposals for skills, `AGENTS.md`, or memory.

### `skill-audit`

Inspect installed skills and compare them with session evidence. Find uncovered rules, scripts that should be added, references that would reduce repeated reasoning, or skill descriptions that should trigger more reliably. Prefer updating an existing Codex skill when coverage is close.

### `technical-skill-finder`

Focus on recurring technical pain: command failures, stack traces, auth/setup loops, package-manager failures, type/lint/test failures, Git/CI/review loops, browser/screenshot automation friction, Obsidian workflows, OpenClaw workspace management as mentioned inside Codex sessions, 1Password flows, GitHub triage, and repeated manual command line rituals. Normalize and cluster signals before recommending reuse, update, or a new skill.

### `proposal`

Produce patch proposals without applying them. Include target path, rationale, evidence, score, privacy/risk notes, and an explicit next action. Patch previews should be small and reviewable.

### `apply-approved`

Apply only patches the user explicitly approved. Re-state the approved target before editing. Keep changes scoped to the approved files, validate frontmatter for skill edits, and show the resulting diff.

## Signal Classes

Mine two disjoint classes of signal.

Behavior and instruction signals:

- repeated user corrections or preference statements
- "continue", "keep going", "don't stop", and similar persistence nudges
- user frustration cues
- repeated style preferences
- autonomy and persistence gaps
- repeated tool-use mistakes
- missing project instructions
- places where project `AGENTS.md` should be updated

Technical skill signals:

- recurring command failures
- stack traces and exception signatures
- authentication, setup, or credential flow failures
- package-manager failures
- type-check, lint, and test failures
- Git, CI, PR review, and review-triage loops
- repeated manual workflows that could become scripts
- recurring local domains such as GitHub triage, browser automation, screenshots, Obsidian, OpenClaw workspace management, 1Password, and similar Codex-driven workflows

Keep these classes separate in the report. A behavior preference may belong in `AGENTS.md`; a repeated failure pattern may belong in a skill or helper script.

## Clustering

Cluster evidence before recommending changes:

- by distinct Codex thread/session, not raw message count
- by date and cwd to avoid inflating same-day retries
- by domain such as Rust, Python, frontend, docs, GitHub, Browser, Obsidian, OpenAI API, or Codex1
- by command lineage such as `cargo test`, `pytest`, `npm install`, `gh pr`, `git worktree`, `python3 script.py`, or Browser automation
- by normalized error signature, not full stack trace text
- by target surface: skill, project instruction, global instruction, memory, or no action

For technical failures, reduce noisy logs into signatures such as `pytest import error`, `cargo clippy lint`, `npm peer dependency`, `gh auth`, `browser blank canvas`, `sqlite schema mismatch`, or `git dirty worktree conflict`.

## Scoring

Use a 0-5 score for each dimension, then turn it into a clear recommendation. Read `references/scoring-and-routing.md` when doing a full ranking pass.

Core dimensions:

- `frequency`: how often the pattern appears
- `recency`: whether it appeared recently
- `impact`: time lost, user frustration, or risk
- `confidence`: how directly the evidence supports the recommendation
- `distinct_sessions`: recurrence across separate threads
- `current_skill_coverage`: whether an existing skill already covers it
- `automation_ease`: whether a script, checklist, or deterministic workflow can help
- `privacy_risk`: whether evidence or proposed text may expose private data
- `target_fit`: whether the fix belongs in a skill, project `AGENTS.md`, global `AGENTS.md`, memory, or no action

High-priority recommendations usually have frequency >= 3, distinct_sessions >= 2, impact >= 3, confidence >= 3, and privacy_risk <= 2. Promote lower-frequency items only when impact is high and the evidence is very clear.

## Routing

Route every recommendation to exactly one primary target:

- `new Codex skill`: repeated technical or workflow pattern with enough scope for a reusable workflow
- `update existing Codex/OpenClaw skill`: existing skill nearly covers the pattern or has a trigger/workflow gap
- `project AGENTS.md`: repo-specific behavior, tooling, domain vocabulary, proof rule, or workflow constraint
- `global ~/.codex/AGENTS.md`: durable preference that applies across most Codex work
- `workspace memory`: durable personal/project context that is useful but not an instruction
- `no action / observe only`: too ambiguous, too private, too rare, already covered, or not worth proceduralizing

Do not put the same rule in both global and project instructions unless there is a clear reason. If the rule is project-specific, keep it project-local. If it is personal preference across repos, consider global instructions. If it is factual context rather than an instruction, route to memory.

For Codex1 repositories, respect the native goal boundary and artifact conventions:

- Codex1 artifacts provide durable mission context and proof; they do not own native goal state.
- Do not start `$clarify`, `$create-prd`, or `$plan` just to run this skill unless the user explicitly asks for a Codex1 mission.
- For Codex1-related recommendations, prefer project `AGENTS.md`, repo-local skills, or docs under `docs/agents/` only when evidence shows a repo-local workflow gap.

## Evidence Rules

Every recommendation must cite:

- session or thread id when available
- timestamp or date
- rollout path or source path
- short evidence summary
- confidence
- why this is not a one-off

Prefer paraphrase over verbatim user or assistant messages. Include private message contents only when necessary, safe, and specifically relevant. Never dump raw full transcripts by default.

Separate facts from inference:

- Facts: "Thread X on 2026-05-20 used `cargo test` three times and failed with the same Rust compile error."
- Inference: "A Rust diagnosis checklist may reduce repeated command churn."
- Proposal: "Update `$diagnose` with a Rust compile-error triage subsection."

## Workflow

1. Confirm scope: mode, timeframe, repo/workspace filters, and whether personal/private logs are explicitly authorized. Personal chat logs are out of scope by default.
2. Run `scan` or manually inventory sources. Prefer SQLite rollout paths when available.
3. Sample sessions conservatively. Start with metadata, then inspect only the cited rollout snippets needed to validate a pattern.
4. Extract behavior/instruction signals and technical skill signals into separate buckets.
5. Cluster by distinct session, domain, command lineage, and error signature.
6. Compare clusters with existing skills by `name`, frontmatter `description`, and relevant `SKILL.md` workflow text.
7. Score and route each recommendation.
8. Produce a proposal report with patch previews, not applied patches.
9. If the user approves specific patches, switch to `apply-approved`, edit only those targets, validate, and show the diff.

## Existing Skill Coverage Check

Inspect skill roots that exist:

- repo-local `skills/*/SKILL.md`
- repo-local `.agents/skills/*/SKILL.md`
- global `~/.codex/skills/*/SKILL.md`
- legacy `~/.agents/skills/*/SKILL.md`
- plugin skills only when they are visible in the current session context or explicitly relevant

Match candidate clusters to existing skills by:

- skill name and aliases
- frontmatter description trigger language
- domain words in headings
- bundled scripts and references
- whether the skill already gives a deterministic workflow for the repeated pain

If a skill exists but did not trigger often enough, propose a frontmatter description update. If the trigger is fine but execution repeated the same manual logic, propose a workflow section, reference file, or script.

## Report Format

Use this practical report shape:

```markdown
# Self Improve Codex Report

## Executive Summary
- analyzed:
- strongest recommendation:
- safest next action:

## Source Coverage
- state db:
- rollout paths:
- fallback sources:
- timeframe:
- limitations:

## Top Recommendations
### 1. <title>
- target: <new skill | existing skill | project AGENTS.md | global AGENTS.md | memory | observe>
- score: <overall> (frequency, recency, impact, confidence, privacy_risk)
- evidence: <thread id/date/path summaries>
- why not one-off:
- suggested patch summary:
- risk/privacy notes:
- next action:

## Observed But Not Actioned
- <cluster>: <reason>
```

Keep reports compact. Load full evidence only when needed to justify a specific recommendation.

## Safety And Privacy

- Never dump full transcripts by default.
- Never print secrets, API keys, tokens, environment dumps, or credential material.
- Treat personal/private chat logs as out of scope unless the user explicitly authorizes that source.
- Prefer paraphrase to raw quotes.
- Default to propose-first.
- Avoid overfitting one-off sessions.
- Keep recommendation buckets disjoint.
- Treat convenience indexes as supporting evidence only.
- If privacy risk is high, recommend no action or a private memory update with redacted wording.
- Before applying approved patches, re-check that the patch does not encode private content as a general instruction.

## Applying Approved Changes

When approval is explicit:

1. Identify the exact approved target and patch intent.
2. Re-read the current target file.
3. Apply the smallest durable edit.
4. Validate skill frontmatter if a `SKILL.md` changed.
5. Run any relevant helper script tests.
6. Show `git diff -- <targets>`.

Never treat a proposal report as approval.
