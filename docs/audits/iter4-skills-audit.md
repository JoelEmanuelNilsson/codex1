# Skills Audit — iter 4

Branch audited: `main` @ `5a16894`
Audited on: 2026-04-20 UTC
Skill folders: `.codex/skills/{clarify,plan,execute,review-loop,close,autopilot}`.
Worktree: `.claude/worktrees/agent-a23c02b0` (no skill edits).

## Scope

For every skill I verified (re-running every iter-3 check against the `5a16894` tree):

- `quick_validate.py` passes (script at `/Users/joel/.codex/skills/.system/skill-creator/scripts/quick_validate.py`).
- Frontmatter has only `name` + `description`; `name` matches the folder name.
- `SKILL.md` body is under 500 lines and written in imperative form.
- `agents/openai.yaml` exists and `interface.default_prompt` references `$<skill-name>` literally.
- No `README.md`, `INSTALLATION_GUIDE.md`, or `CHANGELOG.md` inside the skill folder.
- Body invokes the CLI verbs the skill exists to drive.
- Body does not positively invite behaviors the handoff forbids (caller-identity detection, reviewer writeback, stored waves as editable truth, `.ralph/` as live state, capability/session tokens).

## Summary

**0 P0, 0 P1, 0 P2.** Every skill passes the installed validator, has a clean two-key frontmatter, stays under the 500-line body cap, ships a matching `agents/openai.yaml` with a literal `$<skill-name>` in its `default_prompt`, and contains no forbidden filenames. Bodies invoke the CLI verbs the task prompt requires for each skill. Every handoff anti-goal is either silent (caller identity, session/capability tokens) or positively forbidden in prose.

No findings.

## Build evidence

| Command | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test --release` | 169 passed / 0 failed / 0 ignored |

The skills folder is not exercised by cargo; these are reported for cross-check consistency with the other two iter-4 reports, since the task's build gate applies to the whole audit pass.

## Clean checks (no findings)

### Per-skill `quick_validate.py` output (literal)

```text
=== autopilot ===      Skill is valid!
=== clarify ===        Skill is valid!
=== close ===          Skill is valid!
=== execute ===        Skill is valid!
=== plan ===           Skill is valid!
=== review-loop ===    Skill is valid!
```

### Frontmatter shape (only `name` + `description`)

| Skill | `name:` value | frontmatter keys |
| --- | --- | --- |
| autopilot | `autopilot` | `name`, `description` |
| clarify   | `clarify`   | `name`, `description` |
| close     | `close`     | `name`, `description` |
| execute   | `execute`   | `name`, `description` |
| plan      | `plan`      | `name`, `description` |
| review-loop | `review-loop` | `name`, `description` |

Validator also allows `license`, `allowed-tools`, `metadata`; none are present anywhere. Description fields are single prose blocks under the 1024-character validator ceiling.

### Body length (all under 500 lines)

| Skill | `SKILL.md` line count |
| --- | --- |
| autopilot | 133 |
| clarify | 60 |
| close | 112 |
| execute | 122 |
| plan | 205 |
| review-loop | 81 |

Bodies are imperative (each opens with a verb phrase: `Turn a user's mission goal into…`, `Create a full Codex1 mission plan…`, `Run the next ready task…`, `Orchestrate reviewer subagents…`, `Pause the active Codex1 loop…`, `Take a Codex1 mission from start to terminal close…`).

### `agents/openai.yaml` — `default_prompt` references `$<name>` literally

| Skill | `default_prompt` excerpt | Literal `$<name>` present? |
| --- | --- | --- |
| autopilot   | `Use $autopilot to take a Codex1 mission from clarify to terminal close.` | Yes |
| clarify     | `Use $clarify to fill and ratify OUTCOME.md before planning.` | Yes |
| close       | `Use $close to pause the active loop so the user can talk without Ralph forcing continuation.` | Yes |
| execute     | `Use $execute to run the next ready task or wave for the active Codex1 mission.` | Yes |
| plan        | `Use $plan to scaffold, fill, and lock PLAN.yaml for the active mission.` | Yes |
| review-loop | `Use $review-loop to run reviewer subagents and record clean/dirty findings.` | Yes |

All six yaml files also pin `policy.allow_implicit_invocation: true` as the only policy key.

### No forbidden files inside skill folders

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

### Body invokes the CLI verbs the skill exists to drive

| Skill | CLI verbs mentioned in `SKILL.md` |
| --- | --- |
| clarify | `codex1 init --mission`, `codex1 status --json`, `codex1 outcome check`, `codex1 outcome ratify` |
| plan | `codex1 plan choose-level`, `codex1 plan scaffold`, `codex1 plan check`, `codex1 plan graph`, `codex1 plan waves`, `codex1 replan record` |
| execute | `codex1 task next`, `codex1 task start`, `codex1 task packet`, `codex1 task finish`, `codex1 status --json` |
| review-loop | `codex1 review start`, `codex1 review packet`, `codex1 review record`, `codex1 close check`, `codex1 close record-review` |
| close | `codex1 loop pause`, `codex1 loop resume`, `codex1 loop deactivate`, `codex1 close check`, `codex1 close complete`, `codex1 status` |
| autopilot | `codex1 status --json`, `codex1 init --mission`, `codex1 plan choose-level`, `codex1 close check`, `codex1 close complete` (and composes the other five skills) |

### Handoff anti-goals honored (positive prohibitions where the handoff asks for them)

- **No skill body or reference treats `.ralph/` as a live storage directory.** The one hit inside a skill body is `close/SKILL.md:112`:
  > Do not edit `.ralph/` files, `STATE.json`, or hooks to work around Ralph. Use the `loop` commands.
- **No skill body instructs reviewer subagents to mutate mission truth.** `review-loop/SKILL.md:11` ("The main thread is the sole writer of mission truth"), `review-loop/SKILL.md:63` ("Reviewer writeback is forbidden"), `review-loop/SKILL.md:79` ("Record review results from inside a reviewer subagent — only the main thread records"), and `review-loop/references/reviewer-profiles.md:8-15` (standing reviewer block: `Do not edit files. / Do not invoke Codex1 skills. / Do not run codex1 mutating commands`) all speak the positive prohibition the handoff requires. `execute/SKILL.md:117` reinforces from the orchestrator side: `Do not spawn reviewers, run codex1 review record, or write to reviews/`.
- **No skill body stores waves in `PLAN.yaml` or treats them as editable truth.** `plan/SKILL.md:199` positively forbids it:
  > Do not store waves inside `PLAN.yaml`. Waves are derived.
  `plan/references/dag-quality.md:15` reinforces.
- **No skill asks the CLI to detect caller identity.** No hits for `parent.*subagent|subagent.*parent|caller.*identity|session.*token|capability.*token|authority.*token` inside any `SKILL.md`. `autopilot/SKILL.md` relies on prompt-governed role boundaries; `review-loop/SKILL.md:63-82` enforces reviewer behavior via spawn-prompt prose, not by asking the CLI to gate on identity.
- **Six consecutive dirty reviews triggering replan** is documented at `review-loop/SKILL.md:4` and `execute/SKILL.md:107`, matching `handoff/01-product-flow.md`.

## Reading map — iter 4 skills checklist versus this audit

| iter 4 scope line | Verified in |
| --- | --- |
| `quick_validate.py` on every skill folder | § Per-skill validator output |
| Frontmatter = only `name` + `description` | § Frontmatter shape |
| Body < 500 lines; imperative | § Body length |
| `agents/openai.yaml::default_prompt` mentions `$<skill-name>` literally | § default_prompt references |
| No `README.md` / `INSTALLATION_GUIDE.md` / `CHANGELOG.md` inside skill folders | § No forbidden files |
| Body invokes relevant CLI verbs | § Body invokes the CLI verbs |
| No anti-goal invitations | § Handoff anti-goals honored |

Every iter 4 skills check passes. No source, skill, or doc was modified by this audit.
