# Skills Audit — iter 5

Branch audited: `main` @ `c5e07ad`
Audited on: 2026-04-20 UTC
Skill folders: `.codex/skills/{clarify,plan,execute,review-loop,close,autopilot}`.
Worktree: `.claude/worktrees/agent-a8f65e5b` (no skill edits).

## Scope

For every skill I verified:

- `quick_validate.py` passes (script at `/Users/joel/.codex/skills/.system/skill-creator/scripts/quick_validate.py`).
- Frontmatter has only `name` + `description`; `name` matches the folder name.
- `SKILL.md` body is under 500 lines and written in imperative/descriptive form.
- `agents/openai.yaml` exists and `interface.default_prompt` references `$<skill-name>` literally.
- No `README.md`, `INSTALLATION_GUIDE.md`, or `CHANGELOG.md` inside the skill folder.
- Body invokes the CLI verbs the skill exists to drive.
- Body does not positively invite behaviors the handoff forbids (caller-identity detection, reviewer writeback, stored waves as editable truth, `.ralph/` as live state, capability/session tokens).

## Summary

**0 P0, 0 P1, 0 P2.**

Every skill passes the installed validator, has a clean two-key frontmatter, stays under the 500-line body cap, ships a matching `agents/openai.yaml` with a literal `$<skill-name>` in its `default_prompt`, and contains no forbidden filenames. Bodies invoke the CLI verbs the task prompt requires for each skill. Every handoff anti-goal is either silent (caller identity, session/capability tokens) or positively forbidden in prose. No change since iter 4 — `c5e07ad` only touched `crates/codex1/src/cli/plan/check.rs` and `crates/codex1/tests/plan_check.rs` on top of `5a16894`.

## Build evidence

| Command | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test --release` | 170 passed / 0 failed / 0 ignored |

The skills folder is not exercised by cargo; these are reported for cross-check consistency with the other two iter-5 reports, since the task's build gate applies to the whole audit pass.

## Clean checks

### CC-1: Per-skill `quick_validate.py` output (literal)

```text
$ python3 /Users/joel/.codex/skills/.system/skill-creator/scripts/quick_validate.py .codex/skills/<name>
=== autopilot ===      Skill is valid!
=== clarify ===        Skill is valid!
=== close ===          Skill is valid!
=== execute ===        Skill is valid!
=== plan ===           Skill is valid!
=== review-loop ===    Skill is valid!
```

The validator enforces the two-key frontmatter + name match + description-length ceiling. All six pass.

### CC-2: Frontmatter shape (only `name` + `description`)

| Skill | `name:` value | frontmatter keys |
| --- | --- | --- |
| autopilot   | `autopilot`   | `description`, `name` |
| clarify     | `clarify`     | `description`, `name` |
| close       | `close`       | `description`, `name` |
| execute     | `execute`     | `description`, `name` |
| plan        | `plan`        | `description`, `name` |
| review-loop | `review-loop` | `description`, `name` |

Validator also allows `license`, `allowed-tools`, `metadata`; none are present anywhere. Description fields are single prose blocks under the 1024-character validator ceiling.

### CC-3: Body length (all under 500 lines)

| Skill | `SKILL.md` line count |
| --- | --- |
| autopilot   | 133 |
| clarify     | 60 |
| close       | 112 |
| execute     | 122 |
| plan        | 205 |
| review-loop | 81 |

Bodies read as descriptive-imperative (each opens with a declarative sentence about what the skill does: `$autopilot drives a Codex1 mission …`, `Turn a user's mission goal into a ratified OUTCOME.md …`, `$close is a discussion-mode pause.`, `$execute runs one step at a time against the locked DAG …`, `$plan produces a full mission plan …`, `$review-loop orchestrates reviewer subagents …`). All are under the 500-line cap.

### CC-4: `agents/openai.yaml` — `default_prompt` references `$<name>` literally

| Skill | `default_prompt` | Literal `$<name>` present? |
| --- | --- | --- |
| autopilot   | `Use $autopilot to take a Codex1 mission from clarify to terminal close.` | Yes |
| clarify     | `Use $clarify to fill and ratify OUTCOME.md before planning.` | Yes |
| close       | `Use $close to pause the active loop so the user can talk without Ralph forcing continuation.` | Yes |
| execute     | `Use $execute to run the next ready task or wave for the active Codex1 mission.` | Yes |
| plan        | `Use $plan to scaffold, fill, and lock PLAN.yaml for the active mission.` | Yes |
| review-loop | `Use $review-loop to run reviewer subagents and record clean/dirty findings.` | Yes |

All six yaml files also pin `policy.allow_implicit_invocation: true` as the only policy key.

### CC-5: No forbidden files inside skill folders

```text
$ find .codex/skills -type f \( -iname 'README.md' -o -iname 'INSTALLATION_GUIDE.md' -o -iname 'CHANGELOG.md' \)
(no output)
```

Every skill folder contains only `SKILL.md`, `agents/openai.yaml`, and (for five of six) a `references/` directory:

```text
.codex/skills/
├── autopilot/{agents/openai.yaml, references/autopilot-state-machine.md, SKILL.md}
├── clarify/{agents/openai.yaml, references/outcome-shape.md, SKILL.md}
├── close/{agents/openai.yaml, SKILL.md}
├── execute/{agents/openai.yaml, references/worker-packet-template.md, SKILL.md}
├── plan/{agents/openai.yaml, references/dag-quality.md, references/hard-planning-evidence.md, SKILL.md}
└── review-loop/{agents/openai.yaml, references/reviewer-profiles.md, SKILL.md}
```

`close/` is the one skill without a `references/` directory; its body is self-contained.

### CC-6: Body invokes the CLI verbs the skill exists to drive

Extracted via `grep -oE 'codex1 [a-z]+(-[a-z]+)?( [a-z-]+)?'` against each `SKILL.md`, then filtered to the skill's own scope (mentions of other skills' terminal commands, e.g. `codex1 close complete` inside `execute/SKILL.md:116`, are anti-statements about boundaries and are expected):

| Skill | CLI verbs the skill owns |
| --- | --- |
| clarify     | `codex1 init --mission`, `codex1 status --json`, `codex1 outcome check`, `codex1 outcome ratify` |
| plan        | `codex1 plan choose-level`, `codex1 plan check`, `codex1 replan record` (plus references to `plan scaffold`/`plan graph`/`plan waves` throughout the body) |
| execute     | `codex1 task next`, `codex1 task start`/`finish`/`packet`, `codex1 status --json` |
| review-loop | `codex1 review record`, `codex1 close record-review`, `codex1 close check`, `codex1 task next` |
| close       | `codex1 loop pause`, `codex1 loop resume`, `codex1 loop deactivate`, `codex1 close complete` |
| autopilot   | `codex1 status --json`, `codex1 init --mission`, `codex1 plan choose-level`, `codex1 close check`, `codex1 close complete` (and composes the other five skills) |

Each skill's CLI verb list matches the minimal command surface slice the skill is responsible for.

### CC-7: Handoff anti-goals honored (positive prohibitions where the handoff asks for them)

- **No skill body or reference treats `.ralph/` as a live storage directory.** The one hit inside a skill body is `close/SKILL.md:112`:
  > Do not edit `.ralph/` files, `STATE.json`, or hooks to work around Ralph. Use the `loop` commands.
- **No skill body instructs reviewer subagents to mutate mission truth.** `review-loop/SKILL.md:11` ("The main thread is the sole writer of mission truth"), `review-loop/SKILL.md:63` ("Reviewer writeback is forbidden"), `review-loop/SKILL.md:79` ("Record review results from inside a reviewer subagent — only the main thread records"), and `review-loop/references/reviewer-profiles.md:8-15` (standing reviewer block: `Do not edit files. / Do not invoke Codex1 skills. / Do not run codex1 mutating commands`) all speak the positive prohibition the handoff requires. `execute/SKILL.md:117` reinforces from the orchestrator side: "Do not spawn reviewers, run codex1 review record, or write to reviews/".
- **No skill body stores waves in `PLAN.yaml` or treats them as editable truth.** `plan/SKILL.md:199` positively forbids it:
  > Do not store waves inside `PLAN.yaml`. Waves are derived.
  `plan/references/dag-quality.md:15` reinforces: "Waves are derived by `codex1 plan waves` from `depends_on`."
- **No skill asks the CLI to detect caller identity.** No hits for `parent.*subagent|subagent.*parent|caller.*identity|session.*token|capability.*token|authority.*token` across `.codex/skills/`. `autopilot/SKILL.md` relies on prompt-governed role boundaries; `review-loop/SKILL.md:63-82` enforces reviewer behavior via spawn-prompt prose, not by asking the CLI to gate on identity.
- **Six consecutive dirty reviews trigger replan** is documented at `review-loop/SKILL.md:4` and `execute/SKILL.md:107`, matching `docs/codex1-rebuild-handoff/01-product-flow.md`.

## Reading map — iter 5 skills checklist versus this audit

| iter 5 scope line | Verified in |
| --- | --- |
| `quick_validate.py` on every skill folder | CC-1 |
| Frontmatter = only `name` + `description` | CC-2 |
| Body < 500 lines; imperative | CC-3 |
| `agents/openai.yaml::default_prompt` mentions `$<skill-name>` literally | CC-4 |
| No `README.md` / `INSTALLATION_GUIDE.md` / `CHANGELOG.md` inside skill folders | CC-5 |
| Body invokes relevant CLI verbs | CC-6 |
| No anti-goal invitations | CC-7 |

Every iter 5 skills check passes. No source, skill, or doc was modified by this audit.
