# Round 16 Meta-Review

Baseline under review: `9619b632e07c28be6f32e52e10ca4ff46d8b68c0`

Round 16 was not clean. Sixteen reviewer agents reported raw findings in `docs/audits/round-16/findings.md`. Eight finding-review shards reviewed those candidates. Twenty findings are accepted this round: eleven P1s and nine P2s. The remaining candidates were rejected as below the P0/P1/P2 bar or merged into stronger in-round findings.

Clean-round counter: reset to 0.

## Accepted Findings

| ID | Verdict | Severity | Final title | Notes |
| --- | --- | --- | --- | --- |
| F01 | Accepted | P2 | Spaced `INSTALL_DIR` paths are rewritten, and `verify-installed` falsely passes | Historical custom-install family remains live. |
| F02 | Accepted | P2 | `outcome ratify` can fail after rewriting `OUTCOME.md`, leaving state unratified | Downgraded from P1 because state does not falsely advance, but visible truth still splits on failure. |
| F03 | Accepted | P2 | `outcome check` accepts valid YAML status-key spellings that ratify cannot rewrite | Check/ratify disagreement on valid YAML forms. |
| F04 | Accepted | P1 | Mission-close dirty findings can be immediately marked clean without repair | Direct terminal-gate bypass; F12 merges here. |
| F05 | Accepted | P2 | Terminal loop guard bypasses stale-writer conflict reporting | `--expect-revision` should win before semantic terminal rejection. |
| F06 | Accepted | P1 | Repaired dirty reviews still split status, readiness, and `task next` | Core orchestration surfaces disagree after repair. |
| F07 | Accepted | P1 | `task finish` can mutate stale-plan or terminal work because it uses only the locked-plan guard | F22 merges here. |
| F08 | Accepted | P1 | Close artifacts can publish before state/event commit succeeds | Closeout and mission-close findings artifacts can appear without committed truth. |
| F09 | Accepted | P1 | Mission-close review can be recorded while the locked plan is invalid | Clean mission-close review can be recorded under plan drift. |
| F10 | Accepted | P1 | Restarted planned-review boundaries still accept stale findings from the prior round | Long-running planned-review boundary family remains live. |
| F11 | Accepted | P2 | Late planned-review output during unlocked replan is audited without stable stale category/targets | Current truth is safe, but audit classification is incomplete. |
| F15 | Accepted | P2 | Docs promise externally recorded absolute proof paths remain absolute, but review packets relativize repo-local external proofs | Public reviewer packet contract drift. |
| F16 | Accepted | P2 | Same-sequence event recovery can attach the wrong audit event to a new mutation | Round-15 duplicate-seq fix is under-specified. |
| F17 | Accepted | P1 | Replan relock can orphan active tasks and still allow terminal close | Old non-superseded task truth can be omitted from new DAG and close gate. |
| F18 | Accepted | P1 | Superseded dirty planned-review truth survives replan/relock and blocks the rebuilt DAG | Historical stale dirty-review family remains live. |
| F19 | Accepted | P2 | Hard-plan evidence can be locked without an evidence summary | Mechanical hard evidence gate permits content-free evidence. |
| F20 | Accepted | P1 | Dirty planned-review state commits before the findings artifact is durable | Round-15 artifact-ordering fix inverted the failure mode. |
| F21 | Accepted | P2 | Pure-YAML outcomes render blank closeout summaries | Downstream closeout reader does not support an accepted outcome shape. |
| F23 | Accepted | P2 | Old review outputs after replan relock return `PLAN_INVALID` instead of stale audit | Post-relock stale outputs are dropped rather than audited. |
| F24 | Accepted | P2 | `invalid_state` status can still set `stop.allow: true` | Ralph can fail open on active invalid mission truth. |

## Dropped, Merged, Or Non-Standalone Findings

| ID | Final disposition | Reason |
| --- | --- | --- |
| F12 | Merged into F04 | Same mission-close missing round-identity root cause; F04 is the stronger direct terminal-gate bypass. |
| F13 | Rejected | Phase docs drift is real but was rejected below the P2 bar in round 15 and no new command break was shown. |
| F14 | Rejected | The specific docs-drift claim was disproved; `review status` currently emits PascalCase for target statuses. |
| F22 | Merged into F07 | Same weak `task finish` guard; F07 covers both replan-triggered and terminal facets. |

## Main-Thread Agreement

The main thread accepts the finding-review verdicts with one numbering correction: the event-idempotence issue is accepted as F16, not merged into F01, because F01 in the round-16 finding index is the install-path issue.

Round 16 is not clean for five main reasons:

1. Mutation transaction boundaries still allow visible artifacts and audit log truth to split from committed state:
   - outcome ratify writes `OUTCOME.md` before event/state commit can fail,
   - close and review artifacts can publish on failed mutations,
   - same-seq event recovery can reuse an unrelated trailing event.
2. Review and mission-close boundaries still lack enough identity/freshness:
   - restarted planned reviews accept old findings,
   - stale post-relock outputs are dropped,
   - mission-close dirty can be marked clean immediately.
3. Replan and dirty-review readiness still disagree:
   - repaired dirty reviews keep status/task-next/readiness split,
   - superseded dirty review truth blocks rebuilt DAGs,
   - old active tasks omitted from replacement DAG can be ignored by close.
4. Guard coverage is incomplete:
   - `task finish` skips executable-plan guards,
   - `close record-review` skips locked-plan drift,
   - loop terminal guards bypass stale-revision conflicts,
   - `invalid_state` can still allow Ralph stop.
5. Docs/install/downstream artifact surfaces still drift from supported behavior:
   - spaced install dirs,
   - pure-YAML closeout summaries,
   - repo-local external absolute proofs in review packets,
   - hard evidence entries without summaries.

## Repair Priorities

1. Transaction safety:
   - F02, F08, F16, F20.
2. Execution/readiness/terminal gates:
   - F05, F06, F07, F09, F24.
3. Review/replan/mission-close boundary semantics:
   - F04, F10, F11, F17, F18, F23.
4. Contract and artifact follow-through:
   - F01, F03, F15, F19, F21.
