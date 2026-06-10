# Skills Audit — iter1 (post-6473650)

Branch audited: `audit-iter1-wave` (off `main`) @ `6473650`
Audited on: 2026-04-20 UTC
Skill folders: `.codex/skills/{autopilot,clarify,close,execute,plan,review-loop}`

## Build / test evidence (iter1 header)

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets -- -D warnings` | FAIL (see `iter1-cli-contract-audit.md` P2-1; not a skills issue) |
| `cargo test --release` | PASS — 169 tests across 19 test binaries (steady-state); first run on cold cache flakes tests that invoke `Command::cargo_bin("codex1")`, clears on re-run. |

Clippy failure is unrelated to skill content; scoped to the CLI-contract audit.

## Summary

**0 P0, 0 P1, 0 P2.** Every skill still passes `quick_validate.py`, has exactly `name` + `description` frontmatter, bodies under 500 lines, proper `agents/openai.yaml` with literal `$<skill-name>` default prompts, no forbidden files, invokes the CLI verbs it exists to drive, and honors every declared anti-goal (no `.ralph/` usage, no reviewer writeback, no stored waves, no caller-identity prose).

## Findings

None.

## Per-skill validation (`quick_validate.py`)

```text
=== autopilot ===    Skill is valid!
=== clarify ===      Skill is valid!
=== close ===        Skill is valid!
=== execute ===      Skill is valid!
=== plan ===         Skill is valid!
=== review-loop ===  Skill is valid!
```

Script run: `python3 /Users/joel/.codex/skills/.system/skill-creator/scripts/quick_validate.py <skill-dir>` for each.

## Frontmatter shape

Every `SKILL.md` declares exactly `name` + `description`. `name` matches the folder name.

| Skill | Folder | `name:` in SKILL.md | Other frontmatter keys |
| --- | --- | --- | --- |
| autopilot | `.codex/skills/autopilot/` | `autopilot` | none |
| clarify | `.codex/skills/clarify/` | `clarify` | none |
| close | `.codex/skills/close/` | `close` | none |
| execute | `.codex/skills/execute/` | `execute` | none |
| plan | `.codex/skills/plan/` | `plan` | none |
| review-loop | `.codex/skills/review-loop/` | `review-loop` | none |

`quick_validate.py` accepts `license`, `allowed-tools`, and `metadata`; none appear. Descriptions are single prose blocks with no angle brackets and well under the 1024-character validator ceiling.

## Body length (all < 500 lines)

| Skill | SKILL.md lines |
| --- | --- |
| autopilot | 133 |
| clarify | 60 |
| close | 112 |
| execute | 122 |
| plan | 205 |
| review-loop | 81 |

Unchanged from the baseline audit.

## `agents/openai.yaml` literal `$<skill-name>` references

Every skill ships an `agents/openai.yaml` whose `interface.default_prompt` mentions the skill literally as `$<name>`:

| Skill | `default_prompt` excerpt | Literal `$<name>`? |
| --- | --- | --- |
| autopilot | `Use $autopilot to take a Codex1 mission from clarify to terminal close.` | Yes |
| clarify | `Use $clarify to fill and ratify OUTCOME.md before planning.` | Yes |
| close | `Use $close to pause the active loop so the user can talk without Ralph forcing continuation.` | Yes |
| execute | `Use $execute to run the next ready task or wave for the active Codex1 mission.` | Yes |
| plan | `Use $plan to scaffold, fill, and lock PLAN.yaml for the active mission.` | Yes |
| review-loop | `Use $review-loop to run reviewer subagents and record clean/dirty findings.` | Yes |

All six yaml files also pin `policy.allow_implicit_invocation: true` as the only policy key.

## No forbidden files inside skill folders

```bash
$ find .codex/skills -type f \( -name "README.md" -o -name "INSTALLATION_GUIDE.md" -o -name "CHANGELOG.md" \)
# (no output)
```

Every skill folder contains only `SKILL.md`, `agents/openai.yaml`, and (for five of six) a `references/` directory.

## CLI commands each skill drives

Every skill body invokes the CLI verbs it exists to orchestrate:

| Skill | `codex1` verbs cited in SKILL.md | Notes |
| --- | --- | --- |
| `$clarify` | `codex1 init --mission`, `codex1 status`, `codex1 outcome check`, `codex1 outcome ratify` | Matches mission to ratify OUTCOME.md. |
| `$plan` | `codex1 plan choose-level`, `codex1 plan scaffold`, `codex1 plan check`, `codex1 plan graph`, `codex1 plan waves`, `codex1 replan record --supersedes`, plus status hand-off references. |
| `$execute` | `codex1 status`, `codex1 task next`, `codex1 task start`, `codex1 task packet`, `codex1 task finish` | Explicitly forbids `codex1 review record`. |
| `$review-loop` | `codex1 task next`, `codex1 review start`, `codex1 review packet`, `codex1 review record`, `codex1 close check`, `codex1 close record-review` | Reads `data.proofs` (post-F4 canonical name). |
| `$close` | `codex1 loop pause`, `codex1 loop resume`, `codex1 loop deactivate`, `codex1 close complete`, `codex1 status` | |
| `$autopilot` | `codex1 init`, `codex1 status`, `codex1 plan choose-level`, `codex1 close check`, `codex1 close complete`, plus composes the other five skills. |

## Handoff anti-goals honored

- **No live `.ralph/` directory.** The single skill mention is `close/SKILL.md:112` — a positive prohibition (`- Do not edit .ralph/ files, STATE.json, or hooks to work around Ralph.`). No skill body stores, reads, or creates `.ralph/` state.
- **No reviewer writeback.** `review-loop/SKILL.md:11, 63, 79` all explicitly state "The main thread is the sole writer of mission truth" / "Reviewer writeback is forbidden" / "Record review results from inside a reviewer subagent — only the main thread records." `references/reviewer-profiles.md` restates the ban in the standing reviewer block. `execute/SKILL.md:117` reinforces it from the orchestration side.
- **No stored waves.** `plan/SKILL.md:199` says `Do not store waves inside PLAN.yaml. Waves are derived.` `references/dag-quality.md:15` reinforces. Grep across `.codex/skills/` for `waves:` matches only prose about derivation, never a YAML key emission.
- **No caller-identity semantics.** No skill body asks the CLI to detect caller identity. Role separation is prompt-governed; `review-loop/SKILL.md:77-82` and `autopilot/SKILL.md:120-127` rely on prompt templates rather than CLI checks.
- **Six-consecutive-dirty replan rule is canonical.** `review-loop/SKILL.md:35` ("Inspect `data.replan_triggered`") and `execute/SKILL.md:107` both defer replan decisions to the CLI's counter, not to local bookkeeping.

## Skill folders layout

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

Unchanged from the baseline audit; no new skills were added and none were removed.

## Fresh pass notes (no findings)

1. `review-loop/SKILL.md:21` reads `data.proofs` explicitly, matching the F4 fix in the binary. No skill references the old `target_proofs` field name anywhere. If a skill still referenced `target_proofs`, it would silently break against the post-6473650 CLI; none does.
2. `$close` continues to document both the pause-for-discussion path and the terminal-close path (`close complete`). The handoff-level split between "discussion boundary" and "mission completion" is carried through the skill body's "Terminal close" section (`close/SKILL.md:68-91`), gated on `verdict == mission_close_review_passed` plus explicit user confirmation. Informational, not a finding.
3. `$review-loop` correctly invokes `codex1 close record-review` for the mission-close boundary. Baseline P1-2 (CLI audit) added this verb to the handoff's minimal surface; the skill's invocation is now consistent with the handoff.
