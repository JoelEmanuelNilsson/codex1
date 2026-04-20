# Skills Audit — iter 3

Branch audited: `main` @ `958d2f1`
Audited on: 2026-04-20 UTC
Skill folders: `.codex/skills/{clarify,plan,execute,review-loop,close,autopilot}`.

## Scope

For every skill I verified:

- `quick_validate.py` passes (script at `/Users/joel/.codex/skills/.system/skill-creator/scripts/quick_validate.py`).
- Frontmatter contains only `name` + `description`; `name` matches the folder.
- `SKILL.md` body is under 500 lines and written in imperative form.
- `agents/openai.yaml` exists and `interface.default_prompt` mentions `$<skill-name>` literally.
- No `README.md`, `INSTALLATION_GUIDE.md`, or `CHANGELOG.md` inside the skill folder.
- Body invokes the CLI verbs the skill exists to drive.
- Body does not invite behaviors the handoff forbids (caller-identity, reviewer writeback, stored waves, `.ralph/` as live state).

No skill file was modified between iter 2's audit (`271b2fc`) and this audit (`958d2f1`) — `git diff --stat 271b2fc 958d2f1 -- .codex/skills/` is empty. This audit re-ran every check from scratch regardless.

## Summary

**0 P0, 0 P1, 0 P2.**

Every skill passes validation, has a clean frontmatter, points at the right CLI verbs, and honors the handoff's anti-goals. Nothing regressed between iter 2 and iter 3.

## Findings

None.

## Clean checks

### Per-skill validation (`quick_validate.py`)

All six skills pass the installed validator:

```text
=== autopilot ===    Skill is valid!
=== clarify ===      Skill is valid!
=== close ===        Skill is valid!
=== execute ===      Skill is valid!
=== plan ===         Skill is valid!
=== review-loop ===  Skill is valid!
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

`quick_validate.py` also allows `license`, `allowed-tools`, and `metadata`; none are present. Description fields are single prose blocks with no angle brackets, well under the 1024-character validator ceiling.

### Body length (all < 500 lines)

| Skill | Lines |
| --- | --- |
| clarify | 60 |
| plan | 205 |
| execute | 122 |
| review-loop | 81 |
| close | 112 |
| autopilot | 133 |

Bodies are imperative (each opens with verbs: `Turn a user's mission goal into…`, `Create a full Codex1 mission plan…`, `Run the next ready task…`, `Orchestrate reviewer subagents…`, `Pause the active Codex1 loop…`, `Take a Codex1 mission from start to terminal close…`).

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

All six yaml files pin `policy.allow_implicit_invocation: true` as the only policy key.

### No forbidden files inside skill folders

```bash
$ find .codex/skills -type f \( -name "README.md" -o -name "INSTALLATION_GUIDE.md" -o -name "CHANGELOG.md" \)
# (no output)
```

Each skill folder contains only `SKILL.md`, `agents/openai.yaml`, and (five of six) a `references/` directory.

### CLI commands each skill drives

Every skill body invokes the CLI verbs it exists to orchestrate:

- `$clarify` (`clarify/SKILL.md:19,37,39`): `codex1 status`, `codex1 outcome check`, `codex1 outcome ratify`.
- `$plan` (`plan/SKILL.md:29,45,138,146,147,191`): `codex1 plan choose-level`, `codex1 plan scaffold`, `codex1 plan check`, `codex1 plan graph`, `codex1 plan waves`, `codex1 replan record`.
- `$execute` (`execute/SKILL.md:29,49,50,72`): `codex1 status`, `codex1 task next`, `codex1 task start`, `codex1 task packet`, `codex1 task finish`.
- `$review-loop` (`review-loop/SKILL.md:19,21,28,31,39,43,45`): `codex1 review start`, `codex1 review packet`, `codex1 review record`, `codex1 close check`, `codex1 close record-review`.
- `$close` (`close/SKILL.md:40,51,61,84`): `codex1 loop pause`, `codex1 loop resume`, `codex1 loop deactivate`, `codex1 close complete`, `codex1 status`.
- `$autopilot` (`autopilot/SKILL.md:43,97,104,125`): `codex1 status`, `codex1 init`, `codex1 plan choose-level`, `codex1 close check`, `codex1 close complete`, plus composes the other five skills.

### Handoff anti-goals honored

- **`.ralph/`:** no skill body treats `.ralph/` as live directory. The one hit is `close/SKILL.md:112`, a positive prohibition: `- Do not edit .ralph/ files, STATE.json, or hooks to work around Ralph.` This is the handoff's intended framing.
- **Reviewer writeback:** `review-loop/SKILL.md:11` says "The main thread is the sole writer of mission truth"; `SKILL.md:63` says "Reviewer writeback is forbidden"; `references/reviewer-profiles.md:8-15` lists the standing reviewer block with explicit prohibitions on running `codex1` mutating commands or marking clean anywhere. `execute/SKILL.md:117` reinforces from the orchestrator side.
- **Waves as stored truth:** `plan/SKILL.md:199` says "Do not store waves inside `PLAN.yaml`. Waves are derived." `plan/references/dag-quality.md:15` reinforces.
- **Caller identity:** `autopilot/SKILL.md:120-127` and `review-loop/SKILL.md:77-82` rely on prompt-governed role boundaries. No skill instructs the CLI to check caller identity.
- **Six-dirty-triggers-replan:** documented in `review-loop/SKILL.md:4` and `execute/SKILL.md:107`.

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

References contain prose the SKILL.md bodies intentionally don't duplicate. None repeat SKILL.md content verbatim.
