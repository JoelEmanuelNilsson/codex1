# Native Multi-Agent Resume Note

This note captures what the current qualification evidence does and does not prove
about native child-agent continuity in Codex1.

## Current evidence

The latest qualification report currently fails
`native_multi_agent_resume_flow` in
[.codex1/qualification/latest.json](/Users/joel/codex1/.codex1/qualification/latest.json:430).

That failure is real, but the evidence points more narrowly to a native
support-surface proof gap than to a demonstrated bug in Codex1's parent-led
resume logic.

In the latest failing run:

- the native proof used `spawn_agent`
- the native proof used `wait_agent`
- the native proof did not use `list_agents`
- the native proof did not produce a meaningful live child snapshot before or
  after wait

See the recorded summary fields in
[.codex1/qualification/latest.json](/Users/joel/codex1/.codex1/qualification/latest.json:444).

## Why this matters

Codex1's native child-lane resume contract is parent-led and artifact-led, not
child-led.

Per [docs/MULTI-AGENT-V2-GUIDE.md](/Users/joel/codex1/docs/MULTI-AGENT-V2-GUIDE.md:510):

- V2 does not expose a native `resume_agent` tool
- the parent resumes
- the parent inspects live children
- the parent reconciles expected child task paths
- the parent respawns or reassigns when needed

That means a meaningful qualification proof for native child continuity must
show the exact live child inspection surface required for reconciliation,
especially `list_agents`.

If the trusted build does not surface `list_agents`, or if the proof run does
not successfully gather child visibility/status around the wait/close sequence,
then the gate should fail even if Codex1's internal reconcile code is otherwise
honest.

## What the current failure most likely means

The strongest current hypothesis is:

1. the trusted build or environment did not expose the full expected native
   Multi-Agent V2 surface during the proof run, especially `list_agents`
2. because the proof did not obtain a real live child snapshot, Codex1 had no
   meaningful lane status to reconcile
3. the failing gate therefore does not by itself prove that Ralph-side
   child-lane reconciliation is wrong

This is consistent with:

- the latest failing run in
  [.codex1/qualification/latest.json](/Users/joel/codex1/.codex1/qualification/latest.json:430)
- earlier passing reports where `used_list_agents = true` and
  `wait_summary_present = true`, such as
  [.codex1/qualification/reports/20260413T101021Z--0_120_0--f8b62ac0.json](/Users/joel/codex1/.codex1/qualification/reports/20260413T101021Z--0_120_0--f8b62ac0.json:351)

## What the current failure does not prove

The current report does not strongly prove that:

- Codex1 is falsely concluding child completion
- `resolve-resume` is misclassifying a real live child snapshot
- parent-led reconciliation is conceptually wrong

In the failing run, the resume report still stayed non-terminal rather than
claiming completion, which is the safer behavior.

## Secondary evidence to treat carefully

The same failing report also shows a closeout-write error:

- `internal write-closeout requires governing_revision`

See
[.codex1/qualification/latest.json](/Users/joel/codex1/.codex1/qualification/latest.json:473).

Current source already includes `governing_revision` in the native child
qualification closeout payload at
[crates/codex1/src/commands/qualify.rs](/Users/joel/codex1/crates/codex1/src/commands/qualify.rs:2954).

So that specific error should be treated as likely stale-binary or stale-report
evidence until reproduced against the current source build.

## Fix order

The next investigation and fix order should be:

1. rerun qualification on the current source build and confirm whether the
   closeout-write error still reproduces
2. verify whether the exact trusted build surfaces the expected native V2 child
   tools, especially `list_agents`
3. if `list_agents` is unavailable on that build, treat this as a supported-build
   qualification gap rather than a Codex1 reconcile bug
4. if the full native surface is available and the gate still fails, inspect the
   qualification prompt/expectation and then `resolve-resume` child
   classification against the observed live child snapshot

## Codex1 takeaway

Codex1 should continue to treat native child continuity this way:

- main thread owns mission truth
- child lanes are bounded helpers
- canonical child identity is task-path based
- `wait_agent` is not completion proof
- resume is parent-led and artifact-led

This note therefore narrows the current risk area to:

- proving the exact native V2 support surface on the trusted build
- keeping qualification honest about what that build actually exposes

not:

- redesigning Codex1 around child-owned orchestration
- weakening artifact-led reconciliation
