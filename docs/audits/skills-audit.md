# Skills Audit

Branch audited: `integration/phase-b` @ `a9e9abc`
Audited on: 2026-04-20 UTC
Skill folders: `.codex/skills/{clarify,plan,execute,review-loop,close,autopilot}`.

## Scope

For every skill I verified:

- `quick_validate.py` passes (script at `/Users/joel/.codex/skills/.system/skill-creator/scripts/quick_validate.py`).
- Frontmatter has only `name` + `description` and `name` matches the folder.
- `SKILL.md` body is under 500 lines and written in imperative form.
- `agents/openai.yaml` exists and `interface.default_prompt` references `$<skill-name>` literally.
- No `README.md`, `INSTALLATION_GUIDE.md`, or `CHANGELOG.md` inside the skill folder.
- Body invokes the CLI commands the skill exists to drive.
- Body does not invite behaviors the handoff forbids (caller-identity, reviewer writeback, stored waves, `.ralph/` state).

## Summary

0 P0, 0 P1, 0 P2. Every skill passes validation, has a clean frontmatter, points at the right CLI verbs, and honors the handoff anti-goals. A few observations are noted as informational (no severity) below, but none rise to a contract issue.

## Findings

None.

## Informational (no finding)

### I-1: `$close` invokes `codex1 close complete` — which is outside `$close`'s stated purpose

- **Where:** `.codex/skills/close/SKILL.md:68-91` documents a "terminal close" workflow that calls `codex1 close complete`.
- **Why this is OK (per handoff):** `docs/codex1-rebuild-handoff/01-product-flow.md:196-227` scopes `$close` to "discussion-mode" boundary only, but the file also says "`$close` is not mission completion." The skill body makes the same distinction and explicitly gates `close complete` behind `verdict == mission_close_review_passed` plus user confirmation. Skill is not instructing `$close` to skip mission-close review.
- **Note:** If the product wants to lock `$close` to discussion-only, move the terminal-close documentation to `$autopilot` or a new `$complete` skill. Keeping it documented here but clearly separated is also defensible.

### I-2: `$review-loop` invokes `codex1 close record-review` — tied to P1-2 in the CLI audit

- **Where:** `.codex/skills/review-loop/SKILL.md:43-45`.
- **Note:** The skill is correct relative to the implementation — `close record-review` is the only path to `MissionCloseReviewState::Passed`. The finding belongs to the CLI surface audit (P1-2 in `cli-contract-audit.md`), not to the skill.

## Clean checks (no findings)

### Per-skill validation (`quick_validate.py`)

All six skills pass the installed validator:

```text
=== clarify ===      Skill is valid!
=== plan ===         Skill is valid!
=== execute ===      Skill is valid!
=== review-loop ===  Skill is valid!
=== close ===        Skill is valid!
=== autopilot ===    Skill is valid!
```

### Frontmatter shape

Every `SKILL.md` declares exactly two frontmatter keys, `name` and `description`. The `name` value matches the folder name.

| Skill | Folder | `name:` in SKILL.md | Other frontmatter keys |
| --- | --- | --- | --- |
| clarify | `.codex/skills/clarify/` | `clarify` | none |
| plan | `.codex/skills/plan/` | `plan` | none |
| execute | `.codex/skills/execute/` | `execute` | none |
| review-loop | `.codex/skills/review-loop/` | `review-loop` | none |
| close | `.codex/skills/close/` | `close` | none |
| autopilot | `.codex/skills/autopilot/` | `autopilot` | none |

`quick_validate.py` also allows `license`, `allowed-tools`, and `metadata`; none are present. The description field is a single prose block in each skill, with no angle brackets and well under the 1024-character validator ceiling.

### Body length (all < 500 lines)

| Skill | Lines |
| --- | --- |
| clarify | 60 |
| plan | 205 |
| execute | 122 |
| review-loop | 81 |
| close | 112 |
| autopilot | 133 |

Bodies are imperative (each begins with verbs: `Turn a user's mission goal into…`, `Create a full Codex1 mission plan…`, `Run the next ready task…`, `Orchestrate reviewer subagents…`, `Pause the active Codex1 loop…`, `Take a Codex1 mission from start to terminal close…`).

### `agents/openai.yaml` and `$<skill-name>` literal references

Every skill ships an `agents/openai.yaml` whose `interface.default_prompt` mentions the skill literally as `$<name>`:

| Skill | `default_prompt` excerpt | Literal `$<name>` found? |
| --- | --- | --- |
| clarify | `Use $clarify to fill and ratify OUTCOME.md before planning.` | Yes |
| plan | `Use $plan to scaffold, fill, and lock PLAN.yaml for the active mission.` | Yes |
| execute | `Use $execute to run the next ready task or wave for the active Codex1 mission.` | Yes |
| review-loop | `Use $review-loop to run reviewer subagents and record clean/dirty findings.` | Yes |
| close | `Use $close to pause the active loop so the user can talk without Ralph forcing continuation.` | Yes |
| autopilot | `Use $autopilot to take a Codex1 mission from clarify to terminal close.` | Yes |

All six yaml files also pin `policy.allow_implicit_invocation: true` as the only policy key.

### No forbidden files inside skill folders

```bash
$ find .codex/skills -type f \( -name "README.md" -o -name "INSTALLATION_GUIDE.md" -o -name "CHANGELOG.md" \)
# (no output)
```

Every skill folder contains only `SKILL.md`, `agents/openai.yaml`, and (for five of six) a `references/` directory; the handoff permits references but forbids README/INSTALLATION_GUIDE/CHANGELOG.

### CLI commands each skill drives

Every skill body invokes the CLI verbs it exists to orchestrate:

- `$clarify` (`clarify/SKILL.md:19,37,39`): `codex1 status`, `codex1 outcome check`, `codex1 outcome ratify`. ✓ Matches task requirement that `$clarify` must mention `codex1 outcome check` and `codex1 outcome ratify`.
- `$plan` (`plan/SKILL.md:29,45,138,146,147,191`): `codex1 plan choose-level`, `codex1 plan scaffold`, `codex1 plan check`, `codex1 plan graph`, `codex1 plan waves`, `codex1 replan record`.
- `$execute` (`execute/SKILL.md:29,49,50,72`): `codex1 status`, `codex1 task next`, `codex1 task start`, `codex1 task packet`, `codex1 task finish`.
- `$review-loop` (`review-loop/SKILL.md:19,21,28,31,39,43,45`): `codex1 review start`, `codex1 review packet`, `codex1 review record`, `codex1 close check`, `codex1 close record-review`.
- `$close` (`close/SKILL.md:40,51,61,84`): `codex1 loop pause`, `codex1 loop resume`, `codex1 loop deactivate`, `codex1 close complete`, `codex1 status`.
- `$autopilot` (`autopilot/SKILL.md:43,97,104,125`): `codex1 status`, `codex1 init`, `codex1 plan choose-level`, `codex1 close check`, `codex1 close complete`, plus composes the other five skills.

### Handoff anti-goals honored

- No skill body (or reference) mentions `.ralph/` as a live directory. The one hit is `close/SKILL.md:112`, which is an explicit prohibition: `- Do not edit .ralph/ files, STATE.json, or hooks to work around Ralph.` This is the handoff's intended framing (`.ralph` as anti-goal, not as storage).
- No skill body instructs reviewer subagents to call any mutating CLI command. `review-loop/SKILL.md:11` ("The main thread is the sole writer of mission truth"), `review-loop/SKILL.md:63` ("Reviewer writeback is forbidden"), and `references/reviewer-profiles.md:8-15` (standing instructions: `Do not run codex1 mutating commands`) are all positive-worded prohibitions.
- No skill body stores waves in `PLAN.yaml`. `plan/SKILL.md:199` explicitly says `Do not store waves inside PLAN.yaml. Waves are derived.` `plan/references/dag-quality.md:15` reinforces this.
- No skill body asks the CLI to detect caller identity. `autopilot/SKILL.md:120-127` and `review-loop/SKILL.md:77-82` rely on prompt-governed role boundaries, consistent with the handoff.
- Six consecutive dirty reviews triggering replan is documented in `review-loop/SKILL.md:4` and `execute/SKILL.md:107`.

### Skill folders layout

```
.codex/skills/
├── autopilot/
│   ├── agents/openai.yaml
│   ├── references/autopilot-state-machine.md
│   └── SKILL.md
├── clarify/
│   ├── agents/openai.yaml
│   ├── references/outcome-shape.md
│   └── SKILL.md
├── close/
│   ├── agents/openai.yaml
│   └── SKILL.md
├── execute/
│   ├── agents/openai.yaml
│   ├── references/worker-packet-template.md
│   └── SKILL.md
├── plan/
│   ├── agents/openai.yaml
│   ├── references/dag-quality.md
│   ├── references/hard-planning-evidence.md
│   └── SKILL.md
└── review-loop/
    ├── agents/openai.yaml
    ├── references/reviewer-profiles.md
    └── SKILL.md
```

References contain prose the SKILL.md bodies intentionally don't duplicate — `clarify/references/outcome-shape.md` (field reference table), `plan/references/hard-planning-evidence.md` (spawn prompt templates for explorer/advisor/plan reviewer), `plan/references/dag-quality.md` (DAG heuristics), `execute/references/worker-packet-template.md` (packet substitution template), `review-loop/references/reviewer-profiles.md` (per-profile prompt templates), `autopilot/references/autopilot-state-machine.md` (dispatch table and pseudocode). None repeat SKILL.md content verbatim.
