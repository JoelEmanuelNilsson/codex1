# Codex1 PRD: Native Codex Goals Plus Durable Mission Artifacts

## Problem Statement

Codex1 exists to help Codex keep serious work legible across planning, execution, review, evidence, and handoff. It should not become a second workflow engine.

Native Codex now owns persistent objectives through `/goal` and the official goal tools. That platform layer is responsible for long-running task intent, continuation, pause/resume behavior, time and token accounting, budget limiting, and disciplined completion. Codex1 should not duplicate those responsibilities in mission-local files or hooks.

The user-facing mental model should be simple:

- Native `/goal` means persistent objective and continuation.
- Codex1 means durable artifacts, evidence, receipts, inspection, and mechanical command history.
- Codex remains the semantic judge.

## Solution

Codex1 is a deterministic artifact helper for native Codex workflows.

It creates and manages:

- PRDs;
- plans;
- research plans and research records;
- specs;
- visible subplan lifecycle folders;
- ADRs;
- reviews and triage records;
- proofs;
- closeout records;
- optional receipts;
- mechanical event logs;
- inventory-only inspection;
- doctor diagnostics for the remaining CLI surface.

It does not create, store, mirror, continue, pause, resume, budget, or complete native goals. It does not inspect native goal state. It does not infer readiness, completion, review cleanliness, proof sufficiency, close safety, or next action.

## User Stories

1. As a Codex1 user, I want native `/goal` to own long-running continuation, so that persistent objectives follow the official Codex model.
2. As a Codex1 user, I want Codex1 to remain useful without any native-goal server dependency, so that artifact workflows remain locally testable.
3. As a Codex1 user, I want PRDs and plans to preserve mission intent, so that future Codex sessions do not lose the plot.
4. As a Codex1 user, I want specs and subplans to structure execution, so that implementation slices are clear.
5. As a Codex1 user, I want proofs and closeout to preserve evidence, so that native goal completion has durable human-facing support.
6. As a Codex1 user, I want reviews and triage to remain artifacts, so that reviewer opinions do not become automatic gates.
7. As a Codex1 user, I want receipts to remain optional audit breadcrumbs, so that auxiliary evidence can survive handoff.
8. As a Codex1 user, I want `inspect` to stay inventory-only, so that Codex1 does not become a semantic oracle.
9. As a Codex1 user, I want event logs to record only mechanical CLI mutations, so that event history never becomes workflow truth.
10. As a Codex1 user, I want old continuation files in legacy missions to be ignored, so that stale data cannot steer modern work.
11. As a maintainer, I want the CLI command surface to exclude removed continuation commands, so that users are not guided toward obsolete flows.
12. As a maintainer, I want tests to reject removed command surfaces, so that custom continuation does not creep back in.
13. As a maintainer, I want path containment protections to remain deep and well tested, so that artifact writes stay safe.
14. As a maintainer, I want event-log privacy guarantees to remain tested, so that answer payloads, receipt text, and local paths do not leak.
15. As a future Codex session, I want workflow notes to tell me to use native goal tools for goal state, so that I do not search Codex1 artifacts for continuation truth.

## Implementation Decisions

- Keep the CLI command surface focused on artifacts, subplans, receipts, inspect, templates, init, and doctor.
- Keep JSON envelopes stable for success, warnings, and errors.
- Keep event logging for remaining mechanical mutations only.
- Keep `inspect` inventory-only and explicitly non-semantic.
- Keep path safety as a deep module around contained writes and safe joins.
- Keep template rendering deterministic and versioned.
- Keep subplan lifecycle folders visible and file-based.
- Keep receipts separate from events.
- Remove Codex1-owned continuation state, hook adapters, continuation event kinds, continuation-specific error codes, and diagnostics for deleted behavior.
- Do not add compatibility shims for removed commands.
- Do not migrate legacy continuation files.
- Do not call app-server goal RPCs from Codex1 in this refactor.
- Do not implement native goals inside Codex1.

## Testing Decisions

Good tests assert external behavior:

- removed continuation commands fail through the normal argument parser;
- help output does not advertise removed continuation commands;
- `init --json` reports the current artifact tree and no continuation descriptor;
- new missions do not create legacy continuation files;
- doctor reports only current diagnostics;
- event logs cover initialization, artifact writes, subplan moves, receipts, and safe-layout failures;
- event logs do not include answer payloads, artifact content, receipt messages, absolute paths, or native goal data;
- read-only commands do not append events;
- inspect reports inventory and mechanical warnings only;
- path-safety tests continue for mission roots, artifacts, receipts, subplans, metadata, and event logs;
- docs searches catch stale command instructions.

## Out of Scope

- Implementing official Codex goals inside Codex1.
- Calling app-server goal RPCs from the Codex1 CLI.
- Adding model-tool wrappers for goal creation, inspection, or completion.
- Adding any new Codex1 continuation mechanism.
- Migrating legacy continuation files into native goals.
- Changing native Codex goal semantics.
- Making `inspect` infer readiness, completion, blockers, or next actions.
- Making event logs or receipts into workflow state.
- Redesigning the artifact model beyond the native-goals boundary.

## Further Notes

The healthy shape is smaller than the original product: Codex owns continuation; Codex1 owns durable artifacts and mechanical evidence.

When a user wants long-running work, start a native goal. When Codex needs durable context or proof, write Codex1 artifacts. When work is genuinely done, audit the evidence and then use the native goal completion protocol.
