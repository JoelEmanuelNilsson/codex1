# Round 15 Meta-Review

Baseline under review: `1ac8ca37239194fb70f296c2c61b50bb22de9ce2`

Round 15 was not clean. Sixteen reviewer agents reported raw findings in `docs/audits/round-15/findings.md`. Finding-review shards reviewed the candidate set, and the main thread resolved the final shard directly from repository evidence after no mailbox report was recoverable. Fourteen findings are accepted this round: eight P1s and six P2s. The remaining candidates were rejected, downgraded below the P0/P1/P2 bar, or merged into already-open historical families.

Clean-round counter: reset to 0.

## Accepted Findings

| ID | Verdict | Severity | Final title | Notes |
| --- | --- | --- | --- | --- |
| F01 | Accepted | P1 | Clap-validated argv errors still bypass the JSON envelope / exit-code contract | `Cli::parse()` exits before the canonical envelope path. |
| F03 | Accepted | P2 | Absolute proof paths that point inside the mission bypass mission-local symlink containment | Truly external absolute proofs may remain valid, but mission-local absolute paths must not escape containment rules. |
| F05 | Accepted | P2 | Outcome validation still misses longer placeholder/vague variants like `TODO: fill this in` and `Workflow is reliable.` | The handoff requires no boilerplate placeholders or vague success criteria. |
| F06 | Accepted | P2 | Pure-YAML `OUTCOME.md` files are still rejected even though the primary handoff allows them | Check and ratify must accept both fenced frontmatter and pure YAML mappings. |
| F09 | Accepted | P2 | Replacement plans can reuse untouched historical task IDs from the prior locked DAG snapshot | Replan ID reuse guard ignores IDs that only exist in `state.plan.task_ids`. |
| F10 | Accepted | P1 | `task next` advertises rerunning a dirty review instead of the required repair task | `status` says repair while `task next` returns an unrunnable review. |
| F11 | Accepted | P2 | `plan waves` / `plan graph` advertise executable work while the mission is globally blocked | Read surfaces still show ready work under dirty-review or replan blockers. |
| F12 | Accepted | P1 | Restarting a planned review boundary still accepts stale findings from the previous review round | Old findings can mutate current truth after `review start` restarts the boundary. |
| F13 | Accepted | P2 | Late review results during the unlocked replan window are still rejected before stale audit classification | The stale-output contract requires an audit event instead of a plan-invalid drop. |
| F14 | Accepted | P1 | Dirty planned-review artifacts can publish before the state mutation commits | Review findings files can appear without matching committed state. |
| F15 | Accepted | P1 | `close check` / `close complete` bypass locked-plan drift while `status` reports `invalid_state` | Terminal close must share locked-plan authority with status. |
| F17 | Accepted | P1 | Terminal missions are still mutable through public task and loop commands | `task start` and loop transitions can mutate after `terminal_complete`. |
| F23 | Accepted | P1 | `outcome ratify` can leave `STATE.json` ratified while `OUTCOME.md` is still draft if the file write fails | Round-14 state-first ordering fixed one split-brain shape but introduced the reverse. |
| F24 | Accepted | P2 | Event append-before-state ordering can duplicate `EVENTS.jsonl` sequence numbers after handled post-append failure and retry | The audit log can contain duplicate `seq` values if state persistence fails after append. |

## Dropped, Merged, Or Non-Standalone Findings

| ID | Final disposition | Reason |
| --- | --- | --- |
| F02 | Rejected as standalone | Duplicate of accepted round-14 docs/status-vocabulary drift. |
| F04 | Merge target | Same `CLOSEOUT.md` mission-close history truth family as round-11 F18 / round-12 F18 / round-14 F05. |
| F07 | Rejected | `$clarify` primarily hands off to `$plan`; the lingering `$plan choose-level` phrase is minor wording drift below the P2 bar. |
| F08 | Merge target | Same stale dirty review truth after replan/relock family as round-11 F14 / round-13 F15 / round-14 F08. |
| F16 | Merge target | Same mission-close round-identity family as round-11 F20 / round-12 F21 / round-14 F14. |
| F18 | Merge target | Same post-repair readiness contradiction family as round-11 F11 / round-12 F09. |
| F19 | Merge target | Same dirty-review over-advertising family as F10. |
| F20 | Merge target | Same spaced custom `INSTALL_DIR` family as round-13 F21 / round-14 F19; repair if adjacent install work is hot, but not a standalone round-15 item. |
| F21 | Rejected | The reported `make verify-contract` flake was not reproducible and matches a prior false-positive family. |
| F22 | Rejected | Docs phase wording is stale but not a verified P0/P1/P2 behavioral defect this round. |

## Main-Thread Agreement

The main thread accepts the finding-review verdicts and direct evidence review.

Round 15 is not clean for five main reasons:

1. CLI and path contracts still leak non-canonical behavior:
   - clap parse errors bypass JSON envelopes,
   - absolute mission-local proof paths evade symlink containment.
2. Outcome truth still has parser and atomicity gaps:
   - pure YAML is rejected despite the primary handoff,
   - longer placeholders/vague criteria are missed,
   - ratification can split `STATE.json` and `OUTCOME.md`.
3. Execution/readiness still over-advertises blocked work:
   - dirty reviews are advertised before repair,
   - graph/waves show ready work under global blockers,
   - close bypasses locked-plan drift.
4. Review lifecycle still lacks strong enough boundaries:
   - restarted planned reviews accept stale findings,
   - unlocked-replan late reviews miss stale audit,
   - dirty artifacts can publish before committed truth.
5. State mutation discipline still has two systemic holes:
   - terminal missions are mutable through task/loop commands,
   - event append-before-state can duplicate event sequence numbers after retry.

## Repair Priorities

1. Restore canonical CLI/state mutation discipline:
   - F01, F17, F23, F24.
2. Tighten outcome and proof contracts:
   - F03, F05, F06.
3. Restore blocked-work truthfulness across task/status/graph/close surfaces:
   - F10, F11, F15.
4. Tighten review/replan boundary handling:
   - F09, F12, F13, F14.
