# Round 9 Heavy Review Findings

Date: 2026-04-21

This round restarted the post-round-8 review from scratch with 16 read-only reviewer lanes using `gpt-5.4` with high reasoning. Every reviewer was instructed to read `README.md` and all markdown under `docs/**/*.md`, use prior audit decisions as intended-state context, avoid previously rejected false positives, mutate no repo mission state, and report only verified P0/P1/P2 findings.

Baseline: worktree was clean at commit `31ea3f1` before the round.

## Reviewer Management Table

| Reviewer ID | Surface | Model / reasoning | Result | P0 | P1 | P2 | Finding titles | Evidence quality | Duplicate / unique assessment | Repro status | Main-thread initial disposition | Recommended next step |
| --- | --- | --- | --- | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- |
| R01 | CLI contract and envelopes | gpt-5.4 / high | Findings | 0 | 0 | 1 | `status` advertises close after `close check` blocks on missing proof | High | Duplicates R09/R16 | Reproduced | Candidate P2 | Shard review with close/status items |
| R02 | Mission resolution and path security | gpt-5.4 / high | Findings | 0 | 1 | 0 | `EVENTS.jsonl` / `STATE.json.lock` symlink escape | High | Duplicates part of R10 | Reproduced | Candidate P1 | Shard review with state/path findings |
| R03 | Outcome and clarify | gpt-5.4 / high | Findings | 0 | 0 | 1 | Empty `definitions` / `resolved_questions` still ratify | High | Duplicates R15 | Reproduced | Candidate P2 | Shard review outcome semantics |
| R04 | Plan validation | gpt-5.4 / high | Findings | 0 | 0 | 1 | `plan check` locks before OUTCOME ratification | High | Unique | Reproduced | Candidate P2 | Shard review plan gate |
| R05 | DAG, waves, graph | gpt-5.4 / high | Findings | 0 | 0 | 1 | `plan check` relocks live work depending on superseded task | High | Related to R14 test gap | Reproduced | Candidate P2 | Shard review live-DAG behavior |
| R06 | Task lifecycle | gpt-5.4 / high | Findings | 0 | 0 | 2 | Late review reported as accepted-current; concurrent `task start` not idempotent under lock | High | First overlaps planned-review artifact/current truth; second unique | Reproduced | Candidate P2s | Shard review task/review concurrency |
| R07 | Review lifecycle | gpt-5.4 / high | Findings | 0 | 1 | 0 | Late planned-review findings overwrite current artifact | High | Duplicates R08/R12/R16 | Reproduced | Candidate P1 | Shard review artifact transaction |
| R08 | Replan lifecycle | gpt-5.4 / high | Findings | 0 | 1 | 0 | Late/rejected planned-review writers overwrite current findings artifact | High | Duplicate of R07/R12/R16 | Reproduced | Candidate P1 | Merge with F06 |
| R09 | Close lifecycle | gpt-5.4 / high | Findings | 0 | 0 | 2 | `status` close-ready disagrees with proof-aware `close check`; `close complete` commits terminal before unwritable `CLOSEOUT.md` failure | High | First duplicate R01/R16; second unique closeout ordering | Reproduced | Candidate P2s | Shard review close consistency |
| R10 | State persistence and concurrency | gpt-5.4 / high | Findings | 0 | 2 | 0 | `EVENTS.jsonl` symlink escape; `plan scaffold` creates dirs through symlinked mission root before guard | High | First duplicates R02; second unique pre-guard side effect | Reproduced | Candidate P1s | Shard review path/state |
| R11 | Status and Ralph | gpt-5.4 / high | Findings | 0 | 0 | 1 | Ralph ambiguous-mission jq branch fails because `.ok // empty` masks false | High | Related to R13 fallback issue | Reproduced | Candidate P2 | Shard review hook parsing |
| R12 | Loop commands and orchestration skills | gpt-5.4 / high | Findings | 0 | 1 | 0 | Planned review findings overwritten by rejected stale writer | High | Duplicate R07/R08/R16 | Reproduced | Candidate P1 | Merge with F06 |
| R13 | Install and Makefile UX | gpt-5.4 / high | Findings | 0 | 0 | 1 | Ralph ambiguous-mission fallback fails open when `jq` unavailable | High | Related to R11, separate branch | Reproduced | Candidate P2 | Shard review hook fallback |
| R14 | Test adequacy | gpt-5.4 / high | Findings | 0 | 0 | 5 | Missing tests for superseded-root live-DAG, malformed-plan stale review-start, clean-before-stale-dirty close race, Ralph ambiguous hook, orphan review replan close path | Medium-high | Mostly coverage companions for accepted findings | Suite gap confirmed | Merge into repair tests, not standalone unless issue untested | Add to test plan |
| R15 | Docs/handoff cross-check | gpt-5.4 / high | Findings | 0 | 0 | 1 | Empty required OUTCOME fields still ratify | High | Duplicate R03 | Reproduced | Candidate P2 | Merge with F03 |
| R16 | Current diff / regression | gpt-5.4 / high | Findings | 0 | 2 | 1 | Repair handoff targets cannot be started; late review findings overwrite current artifact; status close-ready after proof deletion | High | Artifact/status duplicates; repair-target start unique | Reproduced | Candidate P1/P2 | Shard review repair lifecycle |

Raw reviewer counts before dedupe: P0 0, P1 8, P2 16.

## Deduplicated Raw Finding Index

Severity here is the reporting-reviewer severity, not final accepted severity.

### F01 P2: `status` advertises close-ready after proof-aware `close check` blocks

Reported by R01, R09, R16. Round 8 made `close check` / `close complete` path-aware for missing proof artifacts, but `status` still uses state-only readiness. Repro: completed task with `proof_path` pointing to a deleted file and mission-close review passed. `status` reports `verdict: mission_close_review_passed`, `close_ready:true`, and `next_action.kind:"close"`; `close check` reports `ready:false` with `PROOF_MISSING`.

Initial disposition: candidate P2. Public projection disagreement.

### F02 P1: `EVENTS.jsonl` and `STATE.json.lock` symlinks let sidecar writes escape `PLANS/<mission>`

Reported by R02/R10. Round-8 write containment rejects symlinked mission roots and artifact parents, but not symlinked artifact files. Repro: replace `EVENTS.jsonl` with a symlink, then run `loop activate`; the event is appended outside the mission. Replace `STATE.json.lock` with a symlink, then `status` creates the outside lock target.

Initial disposition: candidate P1. File-level write containment escape.

### F03 P2: Empty required OUTCOME fields still ratify

Reported by R03/R15. `definitions: {}` and `resolved_questions: []` pass `outcome check` and `outcome ratify`, despite the ratification rule saying no empty required fields and the reference shape requiring definitions and question/answer entries.

Initial disposition: candidate P2. Incomplete round-8 required-field repair.

### F04 P2: `plan check` locks plans before `OUTCOME.md` is ratified

Reported by R04. `plan check` checks revision but not `outcome.ratified`, then can lock a valid `PLAN.yaml` in a fresh unratified mission. Status then shows mixed signals: clarify next action but plan locked and ready tasks.

Initial disposition: candidate P2. Ratified-destination gate bypass.

### F05 P2: `plan check` relocks live work depending on a superseded task

Reported by R05. `plan check` rejects review tasks targeting superseded tasks, but not non-review tasks depending on superseded tasks. Repro: `T1` superseded, live `T2 depends_on [T1]`; `plan check` locks, but `task next` blocks and `plan graph` marks `T2` ready.

Initial disposition: candidate P2. Live-DAG/supersession consistency.

### F06 P1: Late or rejected planned-review writers overwrite the current findings artifact

Reported by R07/R08/R12/R16. `review record --findings-file` copies to `reviews/<id>.md` before locked classification. Late same-boundary or rejected concurrent writers can overwrite the artifact referenced by the current accepted review record while state remains pointed at that path.

Initial disposition: candidate P1. Planned-review analogue of round-8 mission-close artifact race.

### F07 P2: `review record` reports/audits stale `accepted_current` category after locked reclassification makes it audit-only

Reported by R06. `peek_category` is used in the event payload and response, but `apply_record` reclassifies under lock and may treat the record as late/audit-only. Repro with revision bump during large findings copy: response/event says accepted_current dirty while `STATE.json` keeps review pending.

Initial disposition: candidate P2. Related to F06 repair area.

### F08 P2: Concurrent `task start` calls are not idempotent under the state lock

Reported by R06. Two concurrent `task start T1` calls against a ready task both succeed with `idempotent:false`, producing two `task.started` events and revision 2. The idempotent in-progress check is only pre-lock.

Initial disposition: candidate P2. Audit/idempotency contract issue.

### F09 P2: `close complete` commits terminal state before discovering `CLOSEOUT.md` is unwritable

Reported by R09. Repro: ready-to-close mission with `PLANS/demo/CLOSEOUT.md` as a directory. `close complete` returns `PARSE_ERROR`, but state is already terminal, loop deactivated, and retry still fails.

Initial disposition: candidate P2. Incomplete closeout-before-state repair.

### F10 P2: Ralph ambiguous-mission jq branch fails open because `.ok // empty` masks `false`

Reported by R11. The jq guard parses `.ok // empty`; because jq treats `false` as fallback-worthy, `ok` becomes empty and the ambiguous mission error branch never runs. Hook falls through to allow.

Initial disposition: candidate P2. Regression in round-8 Ralph repair.

### F11 P2: Ralph ambiguous-mission fallback fails open when `jq` is unavailable

Reported by R13. The non-jq parser only greps `allow`, so an ambiguous `MISSION_NOT_FOUND` envelope has no `allow` and falls through to allow.

Initial disposition: candidate P2. Same user-visible bug as F10, separate parser branch.

### F12 P2: Round-8 repair coverage gaps remain

Reported by R14. Gaps: superseded-root live-DAG projection across plan/status/task; malformed-plan stale `review start`; clean-before-stale-dirty mission-close race; Ralph ambiguous hook; orphan review-task replan close path still masked by manual state mutation.

Initial disposition: merge as regression-test requirements for accepted findings; likely not a standalone P2 if underlying findings are accepted and tests are added.

### F13 P1: Repair handoff targets cannot be started

Reported by R16. After accepted dirty planned review, `status` emits `next_action.kind:"repair"` with reviewed target ids. Those tasks remain `awaiting_review`, but `task start` only allows `Pending|Ready`, so `$execute` repair handoff stalls with `TASK_NOT_READY`.

Initial disposition: candidate P1. Core review-repair loop break.

## Clean Lanes

No reviewer returned `NONE`; all 16 reported at least one P0/P1/P2 candidate or coverage finding. Round 9 is not clean; clean-round counter remains 0.

