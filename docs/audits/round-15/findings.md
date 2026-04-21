# Round 15 Heavy Review Findings

Date: 2026-04-21

Baseline under review: `1ac8ca37239194fb70f296c2c61b50bb22de9ce2`

This round deployed 16 read-only reviewer lanes using `gpt-5.4` with high reasoning. Every reviewer was instructed to read `README.md` and every markdown file under `docs/**/*.md`, pay special attention to `docs/codex1-rebuild-handoff/` as the primary intended-state source, use prior audit decisions as supporting context, mutate no repo mission state, and report only verified P0/P1/P2 findings after trying to disprove each candidate.

Baseline verification before review:

- `cargo fmt --check` passed.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo test` passed.
- `make verify-contract` passed.

## Summary

- P0: 0
- P1: 20
- P2: 14

Raw reviewer counts before dedupe: 34 findings.

## Reviewer Management Table

| Reviewer ID | Assigned surface | Model | Reasoning | Result | P0 | P1 | P2 | Finding titles | Evidence quality | Duplicate/unique guess | Repro status | Main-thread initial disposition | Recommended next step |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- |
| R01 | CLI contract and envelopes | `gpt-5.4` | high | Findings | 0 | 1 | 1 | Clap argv errors bypass JSON contract; review/task docs drift on status vocabulary | High | First looks unique, second likely docs-only | Reproduced | Candidate P1 + P2 | CLI/docs shard |
| R02 | Mission resolution and path security | `gpt-5.4` | high | Findings | 0 | 1 | 1 | Absolute mission-local proof path bypass; symlinked `reviews/` still poisons closeout history | High | First looks unique, second matches round-14 family | Reproduced | Candidate P1 + merge | Paths/closeout shard |
| R03 | Outcome and clarify | `gpt-5.4` | high | Findings | 0 | 1 | 2 | Placeholder/vague outcome phrases still ratify; pure-YAML outcome rejected; `$clarify` hands off to nonexistent public skill name | High | First two likely unique; third docs/skill drift | Reproduced | Candidate P1 + P2s | Outcome/clarify shard |
| R04 | Plan validation | `gpt-5.4` | high | Findings | 0 | 1 | 1 | Superseded dirty review still blocks rebuilt DAG; replans can reuse untouched historical task IDs from prior `plan.task_ids` | High | First merge family, second likely unique | Reproduced | Candidate P2 + merge | Plan/replan shard |
| R05 | DAG, waves, and graph | `gpt-5.4` | high | Findings | 0 | 1 | 1 | `task next` still resurrects dirty review instead of repair; waves/graph still advertise ready work while mission globally blocked | High | Both match long-open readiness families | Reproduced | Candidate merge set | DAG/readiness shard |
| R06 | Task lifecycle | `gpt-5.4` | high | Findings | 0 | 2 | 1 | Dirty review reruns before repair; restarted review accepts stale prior findings; unlocked-replan late review still `PLAN_INVALID` | High | All match open review/replan families | Reproduced | Candidate merge set | Task/review shard |
| R07 | Review lifecycle | `gpt-5.4` | high | Findings | 0 | 3 | 1 | Restarted review boundary stale findings; superseded dirty review blocks rebuilt mission; unlocked-replan late review still `PLAN_INVALID`; dirty review artifacts publish before commit | High | First three merge families; artifact-publication item likely merge too | Reproduced | Candidate merge set | Review-boundary shard |
| R08 | Replan lifecycle | `gpt-5.4` | high | Findings | 0 | 1 | 1 | Superseded dirty review survives replan; unlocked-replan late review still `PLAN_INVALID` | High | Both match open replan families | Reproduced | Candidate merge set | Replan shard |
| R09 | Close lifecycle | `gpt-5.4` | high | Findings | 0 | 2 | 0 | `close` bypasses locked-plan drift while `status` says invalid_state; mission-close duplicate dirty records can falsely trigger replan | High | First looks unique, second merges mission-close boundary family | Reproduced | Candidate P1 + merge | Close/status shard |
| R10 | State persistence and concurrency | `gpt-5.4` | high | Findings | 0 | 2 | 0 | Terminal missions still mutable through task/loop commands; close artifacts still publish before commit | High | First looks unique, second merge family | Reproduced | Candidate P1 + merge | State/terminal shard |
| R11 | Status and Ralph | `gpt-5.4` | high | Findings | 0 | 2 | 1 | Status stays blocked after repair while re-review is next; `review_required` advertises dirty review too early; superseded dirty review still blocks rebuilt mission | High | First/third merge families; second maybe same family | Reproduced | Candidate merge set | Status/readiness shard |
| R12 | Loop commands and orchestration skills | `gpt-5.4` | high | `NONE` | 0 | 0 | 0 | None | High | No verified issues | N/A | No finding | No action |
| R13 | Install and Makefile UX | `gpt-5.4` | high | Findings | 0 | 0 | 2 | Spaced `INSTALL_DIR` still broken; `make verify-contract` flake on missing `target/debug/codex1` | High | First merge family, second needs extra skepticism | Reproduced once | Candidate P2 + review | Install/verification shard |
| R14 | Test adequacy | `gpt-5.4` | high | Findings | 0 | 2 | 0 | Restarted review stale replay untested; mission-close dirty→immediate clean passes and test suite encodes permissive behavior | High | First merge family; second likely merge/unique depending finding review | Reproduced | Candidate merge set | Test/close shard |
| R15 | Docs and handoff cross-check | `gpt-5.4` | high | Findings | 0 | 0 | 1 | Docs still present `phase` as live review/close state machine, but runtime stays `execute` until terminal | High | Likely unique docs drift | Reproduced | Candidate P2 | Docs/handoff shard |
| R16 | Current diff and regression review | `gpt-5.4` | high | Findings | 0 | 1 | 1 | `outcome ratify` can fail after mutating state; event-before-state ordering can duplicate `EVENTS.jsonl` seq after handled failure | High | Both look like round-14 regression candidates | Reproduced | Candidate P1/P2 | Current-diff/state shard |

## Deduplicated Raw Finding Index

These are raw candidates for finding review. Severity here reflects the reporting reviewer severity, not the final accepted severity.

### F01 P1: Clap-validated argv errors still bypass the published JSON/exit-code contract

Reported by R01.

Evidence:

- `Cli::parse()` still exits directly on clap parse failures before the CLI can emit a `CliError` envelope.
- Missing required args like `review record T4` or `task finish` still produce raw stderr and exit code `2`.
- Public docs still promise stable JSON and reserve exit `2` for harness bugs.

Initial disposition: candidate P1.

### F02 P2: CLI docs still publish stale task/review status vocabulary on public JSON surfaces

Reported by R01.

Evidence:

- `docs/cli-reference.md` still shows PascalCase lifecycle/status strings such as `AwaitingReview`.
- Runtime task/review status strings remain snake_case.

Initial disposition: candidate P2 docs drift.

### F03 P1: Absolute mission-local proof paths still bypass mission-local symlink containment

Reported by R02.

Evidence:

- Relative mission-local proof symlinks are rejected.
- The same mission-local path spelled as an absolute path is accepted because absolute proof resolution skips containment validation.
- `close check` then trusts the absolute mission-local proof path.

Initial disposition: candidate P1.

### F04 P2: `close complete` still trusts a symlinked `reviews/` directory when reconstructing mission-close history

Reported by R02.

Evidence:

- `closeout.rs` still counts `mission-close-*.md` via raw `read_dir(paths.reviews_dir())`.
- External files can still inflate dirty mission-close history in `CLOSEOUT.md`.

Initial disposition: likely merge target into the existing closeout-history family.

### F05 P1: `outcome check` / `ratify` still accept obvious placeholder and vague-success phrasing as long as it is not an exact string match

Reported by R03.

Evidence:

- Placeholder and vague-success checks still rely on exact string matches only.
- `TODO: fill this in`, `Workflow is reliable.`, and similar longer phrases still pass both check and ratify.

Initial disposition: candidate P1.

### F06 P2: The runtime still rejects pure-YAML `OUTCOME.md` files even though the primary handoff explicitly allows them

Reported by R03.

Evidence:

- Handoff says `OUTCOME.md` may be YAML frontmatter plus body, or pure YAML.
- Runtime still unconditionally requires fenced frontmatter.

Initial disposition: candidate P2.

### F07 P2: `$clarify` still hands off to a nonexistent public skill name (`$plan choose-level`)

Reported by R03.

Evidence:

- Public skills surface names only include `$plan`, not `$plan choose-level`.
- Clarify skill still tells future threads to hand off to that nonexistent skill name.

Initial disposition: candidate P2 docs/skill drift.

### F08 P1: Superseded dirty planned-review truth still survives replan/relock and blocks the rebuilt DAG

Reported by R04, R07, R08, and R11.

Evidence:

- `replan record` and relock still leave stale dirty review truth in `state.reviews`.
- `status` / `close check` still block on that stale review even after replacement work is complete.

Initial disposition: likely merge target into the long-open round-11 F14 family.

### F09 P2: Replans can still reuse untouched historical task IDs from the prior locked DAG if those IDs never created `state.tasks`/`state.reviews` rows

Reported by R04.

Evidence:

- Reuse guard only checks IDs already present in `state.tasks` / `state.reviews`.
- IDs present only in prior `plan.task_ids` can still be reused on a replacement plan.

Initial disposition: candidate P2.

### F10 P1: `task next` still advertises rerunning a dirty planned review instead of the required repair step

Reported by R05 and R06.

Evidence:

- `status` correctly says `repair`.
- `task next` still returns `run_review`, and following that advice fails with `REVIEW_FINDINGS_BLOCK`.

Initial disposition: likely merge target into the long-open round-11 F09 family.

### F11 P2: `plan waves` / `plan graph` still advertise a current ready wave / ready nodes even while the mission is globally blocked for repair or replan

Reported by R05.

Evidence:

- Dirty-review and replan-triggered states still produce `current_ready_wave` and `ready` nodes.
- The corresponding mutating commands fail closed.

Initial disposition: likely merge target into open readiness families.

### F12 P1: Restarting a planned review boundary still does not fence off stale findings from the previous review round

Reported by R06, R07, and R14.

Evidence:

- A second `review start` creates a new boundary.
- Replaying old findings after restart still yields `accepted_current` and increments dirty counters.

Initial disposition: likely merge target into the long-open round-11 F19 family.

### F13 P2: Late review results during the unlocked replan window are still rejected as `PLAN_INVALID` instead of being stale-audited

Reported by R06, R07, and R08.

Evidence:

- `review record` still enforces plan-lock before stale/superseded classification.
- Late replay after `replan record` still drops the stale audit event.

Initial disposition: likely merge target into the long-open round-10 F14 / round-11 merged F15 family.

### F14 P1: Dirty planned-review artifacts still publish before the review mutation commits

Reported by R07.

Evidence:

- Dirty `review record` still writes `reviews/<id>.md` before the event/state commit succeeds.
- Failure after artifact publication still leaves visible review files with unchanged state.

Initial disposition: likely merge target into the long-open artifact-publication family.

### F15 P1: `close check` / `close complete` still bypass locked-plan drift and can terminalize a mission that `status` already marks `invalid_state`

Reported by R09.

Evidence:

- `status` now checks locked-plan snapshot drift and can emit `invalid_state`.
- `close check` / `close complete` do not run the same locked-plan hash guard.
- A mutated post-lock plan can still close and terminalize the mission.

Initial disposition: candidate P1. Looks unique and high-signal.

### F16 P1: Mission-close review still has no round identity, so duplicate dirty `close record-review` calls can falsely trigger replan without any repair/review round progression

Reported by R09 and reinforced by R14.

Evidence:

- Mission-close dirty records still increment the dirty counter on every duplicate write.
- Repeating the same dirty record six times can still set `replan.triggered=true`.

Initial disposition: likely merge target into the long-open round-11 F20 family.

### F17 P1: Terminal missions are still mutable through public task/loop commands

Reported by R10.

Evidence:

- `task start` and loop commands still lack terminal guards.
- A terminal mission can still mutate task and loop state while `status` keeps reporting `terminal_complete`.

Initial disposition: candidate P1.

### F18 P1: `status` can still say `blocked` after repair even when the next runnable action is the re-review

Reported by R11.

Evidence:

- Dirty review remains an unconditional `blocked` verdict in readiness.
- Once repair completes, `status.next_action` can still become `run_review`, producing an internally contradictory payload that current autopilot cannot act on.

Initial disposition: likely merge target into the long-open round-10 F11 / round-12 F09 family.

### F19 P2: `status.review_required` still advertises a dirty review as ready before the required repair has happened

Reported by R11.

Evidence:

- `status.review_required` still lists the dirty review task.
- `review start` on that same task still fails with `REVIEW_FINDINGS_BLOCK`.

Initial disposition: likely merge target into the dirty-review-overadvertised family.

### F20 P2: Spaced `INSTALL_DIR` values are still broken, and `verify-installed` can still pass against the wrong path

Reported by R13.

Evidence:

- `INSTALL_DIR_ABS := $(abspath $(INSTALL_DIR))` still splits whitespace-containing paths.
- `install-local` / `verify-installed` still certify a malformed path, not the requested install dir.

Initial disposition: likely merge target into the long-running install-dir family.

### F21 P2: `make verify-contract` still appears flaky because `cargo test` can sometimes lose `target/debug/codex1`

Reported by R13.

Evidence:

- Reviewer observed a single `verify-contract` failure where an integration test could not find `target/debug/codex1`.
- Immediate reruns passed.

Initial disposition: needs extra skepticism in finding review; possible false positive from concurrent local build/test interference.

### F22 P2: Docs still present `phase` as a live review/mission-close state machine, but the runtime stays `execute` until terminal

Reported by R15.

Evidence:

- Docs still publish `execute -> review_loop -> mission_close -> terminal`.
- Runtime still only transitions `clarify -> plan -> execute -> terminal`.
- Review/mission-close progression is actually represented by verdict/next_action, not phase.

Initial disposition: candidate P2 docs drift.

### F23 P1: `outcome ratify` can now fail after already mutating mission state, leaving `STATE.json` and `OUTCOME.md` split

Reported by R16.

Evidence:

- Round-14 ratify path now mutates state before rewriting `OUTCOME.md`.
- A forced write failure after the state mutation leaves `STATE.json` ratified/plan-phase while `OUTCOME.md` still says `status: draft`.

Initial disposition: candidate P1 regression.

### F24 P2: The new event-before-state mutation ordering can still leave duplicate `EVENTS.jsonl` sequence numbers after a handled write failure and retry

Reported by R16.

Evidence:

- Event append now occurs before state persistence.
- A handled failure after event append but before state write leaves `STATE.json.events_cursor` unchanged with an already-appended event.
- Retrying the same command appends a second event with the same `seq`.

Initial disposition: candidate P2 regression / transaction-order issue.

## Clean Lanes

- R12 `Loop commands and orchestration skills` returned `NONE`.
