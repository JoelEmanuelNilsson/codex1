# Iter2 Skills Audit

Branch audited: `audit-iter2-wave` (worktree off `main` @ `271b2fc`).
Audit iteration: iter 2 (after iter1 fix at `6473650` and the clippy follow-up at `271b2fc`).
Audited on: 2026-04-20 UTC.
Skill folders: `.codex/skills/{autopilot, clarify, close, execute, plan, review-loop}` (six skills).

## Build evidence

| Command | Result |
| --- | --- |
| `cargo fmt --check` | PASS (no diff) |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test --release` | PASS — 169 passed / 0 failed / 0 ignored across 18 binaries |

## Scope

For each of the six skills I re-verified at commit `271b2fc`:

1. `quick_validate.py` (installed at `/Users/joel/.codex/skills/.system/skill-creator/scripts/quick_validate.py`) reports valid.
2. Frontmatter contains only `name` + `description`, and `name` matches the folder.
3. SKILL.md body is < 500 lines and written in imperative form.
4. `agents/openai.yaml` ships and `interface.default_prompt` mentions `$<skill-name>` literally.
5. No `README.md`, `INSTALLATION_GUIDE.md`, or `CHANGELOG.md` inside the skill folder.
6. The body invokes the CLI commands the skill exists to drive.
7. The body does not invite any handoff anti-goal (caller-identity, reviewer writeback as authority, stored waves, `.ralph/` truth, semantic CLI questionnaire, hidden daemons, etc.).

## Summary

**0 P0 / 0 P1 / 0 P2.** Tree is clean.

## Findings

No findings.

## Clean checks (every check passed)

### CC-1: `quick_validate.py` — all six skills valid

```bash
$ for s in autopilot clarify close execute plan review-loop; do
    python3 /Users/joel/.codex/skills/.system/skill-creator/scripts/quick_validate.py \
      .codex/skills/$s | tail -1
  done
Skill is valid!     # autopilot
Skill is valid!     # clarify
Skill is valid!     # close
Skill is valid!     # execute
Skill is valid!     # plan
Skill is valid!     # review-loop
```

### CC-2: Frontmatter — `name` + `description` only, names match folders

Every SKILL.md ships exactly two frontmatter keys (`name`, `description`). The `name` field equals the folder name. Verified by reading the YAML block of each file:

| Skill | Folder | `name:` value | Other top-level keys |
| --- | --- | --- | --- |
| autopilot | `.codex/skills/autopilot/` | `autopilot` | none |
| clarify | `.codex/skills/clarify/` | `clarify` | none |
| close | `.codex/skills/close/` | `close` | none |
| execute | `.codex/skills/execute/` | `execute` | none |
| plan | `.codex/skills/plan/` | `plan` | none |
| review-loop | `.codex/skills/review-loop/` | `review-loop` | none |

`quick_validate.py` permits `license`, `allowed-tools`, `metadata`; none of these are set. Each `description` is well below the 1024-character validator ceiling.

### CC-3: SKILL.md body length — every body under 500 lines, imperative voice

```bash
$ wc -l .codex/skills/*/SKILL.md
   133 .codex/skills/autopilot/SKILL.md
    60 .codex/skills/clarify/SKILL.md
   112 .codex/skills/close/SKILL.md
   122 .codex/skills/execute/SKILL.md
   205 .codex/skills/plan/SKILL.md
    81 .codex/skills/review-loop/SKILL.md
```

All six bodies are well under 500 lines (max 205, min 60). Each begins with an imperative phrase: `Take a Codex1 mission ...`, `Ratify OUTCOME.md ...`, `Pause the active Codex1 loop ...`, `Run the next ready task ...`, `Create a full Codex1 mission plan ...`, `Run reviewer subagents ...`.

### CC-4: `agents/openai.yaml` — `default_prompt` mentions `$<skill-name>` literally

Every skill ships `agents/openai.yaml`. Each `interface.default_prompt` literally contains the `$<name>` token:

| Skill | `default_prompt` | `$<name>` literal present |
| --- | --- | --- |
| autopilot | `Use $autopilot to take a Codex1 mission from clarify to terminal close.` | yes (`$autopilot`) |
| clarify | `Use $clarify to fill and ratify OUTCOME.md before planning.` | yes (`$clarify`) |
| close | `Use $close to pause the active loop so the user can talk without Ralph forcing continuation.` | yes (`$close`) |
| execute | `Use $execute to run the next ready task or wave for the active Codex1 mission.` | yes (`$execute`) |
| plan | `Use $plan to scaffold, fill, and lock PLAN.yaml for the active mission.` | yes (`$plan`) |
| review-loop | `Use $review-loop to run reviewer subagents and record clean/dirty findings.` | yes (`$review-loop`) |

All six yaml files set `policy.allow_implicit_invocation: true` as the only policy key.

### CC-5: No forbidden files inside skill folders

```bash
$ ls .codex/skills/**/{README.md,INSTALLATION_GUIDE.md,CHANGELOG.md} 2>/dev/null
# (no output — Glob '.codex/skills/**/{README.md,INSTALLATION_GUIDE.md,CHANGELOG.md}' returned no files)
```

Every skill folder contains exactly: `SKILL.md`, `agents/openai.yaml`, and (for five of six) a `references/` directory. No README, INSTALLATION_GUIDE, or CHANGELOG. Layout:

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
│   └── SKILL.md             # no references/ — close is small enough not to need one
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

### CC-6: Body invokes the CLI commands each skill drives

Confirmed by grepping each SKILL.md for `codex1 <verb>` invocations:

- `clarify/SKILL.md` (`codex1 status`, `codex1 outcome check`, `codex1 outcome ratify`) — drives `outcome` group end-to-end.
- `plan/SKILL.md` (`codex1 plan choose-level`, `codex1 plan scaffold`, `codex1 plan check`, `codex1 plan graph`, `codex1 plan waves`, `codex1 replan record`) — drives `plan` + `replan record` for replan branch.
- `execute/SKILL.md` (`codex1 status`, `codex1 task next`, `codex1 task start`, `codex1 task packet`, `codex1 task finish`) — drives `task` lifecycle.
- `review-loop/SKILL.md` (`codex1 review start`, `codex1 review packet`, `codex1 review record`, `codex1 close check`, `codex1 close record-review`) — drives `review` + `close record-review`.
- `close/SKILL.md` (`codex1 loop pause`, `codex1 loop resume`, `codex1 loop deactivate`, `codex1 close complete`, `codex1 status`, `codex1 close check`) — drives `loop` + the terminal-close branch of `close`.
- `autopilot/SKILL.md` + `references/autopilot-state-machine.md` (`codex1 status`, `codex1 init`, `codex1 plan choose-level`, `codex1 close check`, `codex1 close complete`, `codex1 loop pause`, `codex1 loop resume`) — composes the other five skills around `codex1 status --json`.

### CC-7: Handoff anti-goals are honored (positive prohibitions present)

- **Reviewer writeback forbidden.** `review-loop/SKILL.md:11` ("The main thread is the sole writer of mission truth"); `review-loop/SKILL.md:63` ("Reviewer writeback is forbidden"); `review-loop/SKILL.md:79` ("Record review results from inside a reviewer subagent — only the main thread records"); `references/reviewer-profiles.md:8-15` (standing reviewer block: "Do not run codex1 mutating commands"). `execute/SKILL.md:117` mirrors from the orchestration side ("Do not spawn reviewers, run `codex1 review record`, or write to `reviews/`").
- **Stored waves.** `plan/SKILL.md:199` — "Do not store waves inside `PLAN.yaml`. Waves are derived." `plan/references/dag-quality.md:15` reinforces ("Waves are derived by `codex1 plan waves` from `depends_on`."). No skill body or reference emits a `waves:` YAML key.
- **`.ralph/` mission truth.** Only mention is `close/SKILL.md:112` ("Do not edit `.ralph/` files, `STATE.json`, or hooks to work around Ralph. Use the `loop` commands.") — a positive prohibition.
- **Caller identity.** No skill body references `is_parent`, `is_subagent`, `caller_type`, `reviewer_id`, `session_id`, `session_token`, `capability_token`, or `authority_token`. Role boundaries are prompt-governed (`autopilot/SKILL.md:120-127`, `review-loop/SKILL.md:77-82`).
- **Six consecutive dirty triggers replan.** Documented in `review-loop/SKILL.md:35` (replan handoff via `data.replan_triggered`) and `execute/SKILL.md` (defers to review-loop). The threshold is enforced by `codex1 review record` / `codex1 close record-review`, not by the skill.
- **Mission-close terminal close path.** `close/SKILL.md:12-30` separates discussion-mode (`loop pause`/`resume`) from terminal close (`close complete`). The terminal path is gated on `verdict == mission_close_review_passed` plus user confirmation.

### CC-8: References directories — supplementary prose, not duplicate body

| Reference | Purpose |
| --- | --- |
| `clarify/references/outcome-shape.md` | OUTCOME.md required-field reference. |
| `plan/references/dag-quality.md` | DAG design heuristics for non-trivial plans. |
| `plan/references/hard-planning-evidence.md` | Spawn templates for explorer/advisor/plan-reviewer subagents. |
| `execute/references/worker-packet-template.md` | Worker spawn template substituted from `task packet` envelope. |
| `review-loop/references/reviewer-profiles.md` | Standing reviewer instructions + per-profile spawn templates. |
| `autopilot/references/autopilot-state-machine.md` | Verdict→skill dispatch table and pseudocode. |

None of the references duplicate SKILL.md content verbatim. None invite anti-goals.

### CC-9: Skill bodies do not invite semantic CLI questions

The handoff forbids the CLI from asking semantic clarification questions (`02-cli-contract.md:112-119`). The skills route all clarification through `$clarify`, which interviews the user and writes OUTCOME.md — the CLI only validates and ratifies. `clarify/SKILL.md` confirms this division: it interviews on the main thread, then invokes `codex1 outcome check` / `codex1 outcome ratify` to close.

No skill instructs the CLI to ask for semantic input.

### CC-10: Skill bodies are imperative

Each SKILL.md opens with an imperative verb in the description (CC-3). Internal section headings ("Required workflow", "Preconditions", "Do not", "Notes") are also imperative. No body asks open-ended questions of the reader; each step is a numbered action with a concrete CLI/skill call.

## Notes (informational, not findings)

- `$close/SKILL.md` documents two distinct workflows (discussion-mode `loop pause`/`resume`/`deactivate` and terminal-close `close complete`). The terminal-close path is gated on `mission_close_review_passed` + user confirmation. The handoff scopes `$close` to discussion mode (`01-product-flow.md:196-227`) but explicitly notes "`$close` is not mission completion." The skill body honors this distinction. No finding.
- `$review-loop` invokes `codex1 close record-review`, which is the only write path to `MissionCloseReviewState::Passed`. Documented in `docs/cli-contract-schemas.md:320-337` and listed in the handoff minimal surface (`02-cli-contract.md:93`). No finding.
