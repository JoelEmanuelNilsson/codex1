# Scoring And Routing Reference

Use this reference during `dream`, `skill-audit`, `technical-skill-finder`, and `proposal` modes when a full ranking pass is needed.

## Scoring Dimensions

Score each dimension from 0 to 5.

| Dimension | 0 | 3 | 5 |
| --- | --- | --- | --- |
| `frequency` | one occurrence | 2-3 related occurrences | frequent repeated pattern |
| `recency` | old or stale | seen in recent months | seen in recent weeks |
| `impact` | cosmetic or low effort | noticeable rework or friction | repeated user frustration, risk, or major time loss |
| `confidence` | weak inference | evidence points to pattern | direct repeated evidence |
| `distinct_sessions` | one thread | two distinct sessions | many distinct sessions/projects |
| `current_skill_coverage` | already covered well | partial coverage | no effective coverage |
| `automation_ease` | cannot automate | checklist/reference helps | deterministic script/workflow likely helps |
| `privacy_risk` | no private content | manageable with paraphrase | high risk; avoid or redact |
| `target_fit` | unclear target | plausible target | obvious single target |

For `current_skill_coverage`, score high when coverage is missing and low when an existing skill already handles the issue. If an existing skill is close, prefer an update over a new skill.

## Overall Priority

Use judgment, but this weighted shape is a good default:

```text
priority =
  frequency * 1.2
  + recency * 0.8
  + impact * 1.4
  + confidence * 1.4
  + distinct_sessions * 1.1
  + current_skill_coverage * 0.8
  + automation_ease * 0.7
  + target_fit * 0.8
  - privacy_risk * 1.5
```

Priority bands:

- `>= 28`: strong recommendation
- `22-27`: good candidate; propose if evidence is clean
- `16-21`: observe or include as secondary
- `< 16`: no action unless user specifically asks

Promote a lower score only when impact is high and evidence is direct. Demote anything with privacy risk >= 4 unless the output can be safely redacted.

## Routing Matrix

Choose one primary target.

| Pattern | Primary Target | Notes |
| --- | --- | --- |
| repeated tool or workflow failure across projects | new Codex skill | use when existing skills do not cover the domain |
| repeated issue inside a known skill's domain | update existing Codex/OpenClaw skill | update trigger text, workflow, reference, or script |
| repo-specific command, proof, domain, or artifact rule | project `AGENTS.md` | nearest repo-local instructions only |
| durable preference across most work | global `~/.codex/AGENTS.md` | avoid project-only details |
| durable fact/context, not an instruction | workspace memory | keep it factual and scoped |
| one-off, ambiguous, private, or already covered | no action / observe only | record the reason |

## Evidence Quality

Strong evidence includes:

- two or more distinct Codex sessions
- recent recurrence
- source paths from SQLite rollout metadata
- matching command lineage or normalized error signatures
- user correction or frustration repeated in more than one context

Weak evidence includes:

- one session only
- one long thread with repeated retries
- inferred preferences with no explicit user correction
- convenience index entries without rollout validation
- raw transcript content that cannot be safely paraphrased

## Technical Signal Normalization

Normalize logs into stable signatures:

- `auth`: `gh auth`, SSH key, 1Password, API key, OAuth, permission denied
- `package-manager`: `npm`, `pnpm`, `yarn`, `pip`, `uv`, `cargo`, lockfile or resolver failures
- `type-check`: TypeScript, Rust compile, Python typing, schema validation
- `lint-format`: formatter/linter failure, style churn
- `test`: failing test command, flaky test, missing fixture
- `runtime`: exception, stack trace, crash, browser console error
- `git-ci-review`: merge conflict, dirty worktree, branch, PR, CI, review triage
- `browser-visual`: screenshot, blank canvas, viewport, asset loading, DOM overlap
- `local-workflow`: Obsidian, Codex worktrees, OpenClaw workspace management, local app automation

Cluster by command lineage first, then error signature, then domain. Do not create separate candidates for the same failure with different wording.

## Proposal Patch Shape

Every proposal should contain:

- target path or target class
- one-sentence rationale
- score and key dimension values
- evidence anchors with thread id/date/source path
- why it recurs across distinct sessions
- exact patch summary
- privacy note
- recommended next action

Patch previews should be minimal. If a candidate needs more design work, propose a first iteration instead of drafting a giant skill.
