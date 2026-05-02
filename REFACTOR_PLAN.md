# Codex1 Refactor Plan: Native Goals Boundary

## Status

This document supersedes the older setup-and-hook activation plan. Codex1 no longer plans to install or depend on a custom continuation hook. Native Codex `/goal` owns persistent objectives and continuation.

## Problem Statement

Codex1 previously carried product planning for a setup bundle that included a custom continuation hook and mission-local continuation state. That direction conflicts with native Codex goals.

The refactor goal is to make Codex1 smaller and clearer:

- native `/goal` owns persistent objectives, continuation, pause/resume, accounting, budgets, and completion;
- Codex1 owns durable artifacts, receipts, inspection, doctor diagnostics, and mechanical event logs;
- Codex remains the semantic judge;
- legacy continuation files are ignored, not migrated.

## Solution

Remove Codex1-owned continuation behavior and update code, tests, and docs around the native-goals boundary. Preserve setup only as repo-scoped Codex1 skill/guidance materialization with backups and mechanical status.

The CLI should expose only current mechanical commands:

- `init`;
- `template list`;
- `template show`;
- `interview`;
- `subplan move`;
- `receipt append`;
- `inspect`;
- `setup install`, `setup enable`, `setup disable`, `setup uninstall`, `setup status`, `setup doctor`, and setup backups;
- `doctor`.

Removed command surfaces should fail through the normal argument parser. There should be no compatibility wrappers and no replacement continuation subsystem.

## Implementation Checklist

1. Record the current build/test baseline before editing.
2. Remove continuation module registrations from the binary.
3. Delete continuation command type definitions.
4. Delete hook-adapter command type definitions.
5. Remove dispatch branches for deleted command surfaces.
6. Delete the command handlers for removed behavior.
7. Delete the mission-local continuation state model and read/write helpers.
8. Delete the hook adapter implementation.
9. Remove continuation-specific error classification.
10. Remove continuation event kinds and constructors.
11. Remove continuation state descriptors and layout helpers.
12. Simplify doctor diagnostics to current CLI mechanics.
13. Remove dead helpers revealed by doctor simplification.
14. Reduce setup to repo-local skill/guidance materialization and remove all hook/Ralph/global-policy/migration behavior.
15. Add or update CLI rejection tests for removed command surfaces.
16. Update initialization tests so new missions have no continuation descriptor or state file.
17. Update inspect tests to preserve inventory-only behavior without continuation output.
18. Update event-log tests to cover remaining mutation families only.
19. Update read-only command tests to cover current read-only commands.
20. Preserve path-safety tests for remaining writable surfaces.
21. Add setup tests for install/status/disable/enable/backups and removed hook options.
22. Update README, CLI contract, artifact model, skill workflow notes, PRD, and planning docs.
23. Run formatting.
24. Run the full test suite.
25. Run a final stale-reference search for removed command instructions, legacy state filename, removed event kinds, removed error code, and setup hook language.
26. Manually smoke the remaining CLI surface.

## Decision Document

- Codex1 will not own a long-running continuation primitive.
- Native Codex goals are the continuation and long-running-task primitive.
- Codex1 will not reimplement official goal persistence, runtime accounting, automatic continuation, token budgets, pause/resume behavior, or completion semantics.
- Codex1 will not call official goal RPCs in this refactor.
- Codex1 remains a deterministic artifact helper.
- Mission-local event logs remain mechanical observability only.
- Event logs do not record continuation state because Codex1 no longer mutates it.
- Doctor tests remaining Codex1 mechanics, not native Codex goal behavior.
- Legacy `.codex1/LOOP.json` files may remain on disk in old missions, but Codex1 no longer reads, writes, scans, blocks on, or migrates them.
- Removed commands fail through the standard CLI argument parser.
- Documentation teaches `/goal` for persistent objective tracking and Codex1 artifacts for structure, evidence, and handoff.
- Setup focuses on artifact workflow guidance and skill materialization, not continuation hooks.

## Testing Decisions

- CLI tests verify removed command surfaces are unavailable through normal parser behavior.
- Mission initialization tests verify the remaining mission structure without continuation state.
- Inspect tests verify inventory and mechanical warnings only.
- Event-log tests cover remaining mutating commands and prove removed continuation events are not part of the contract.
- Read-only command tests cover current read-only commands only.
- Doctor tests assert current diagnostics and a stable success envelope.
- Path-safety tests stay focused on artifacts, subplans, receipts, metadata, and event logs.
- Documentation verification includes repository searches for stale command instructions and stale product language.
- Final verification includes formatting, the full test suite, a targeted stale-reference search, and a manual smoke of current CLI flows.

## Out of Scope

- Implementing official Codex goals inside Codex1.
- Adding app-server RPCs or protocol clients to Codex1.
- Adding wrappers for goal creation, inspection, or completion.
- Recreating removed continuation behavior on top of native goals.
- Maintaining compatibility shims for removed commands.
- Migrating legacy state files.
- Automatically creating native goals from legacy files.
- Editing the official Codex repository.
- Changing native Codex goal semantics.
- Building setup flows for continuation hooks.
- Broad artifact-model redesign unrelated to the native-goals boundary.
- Semantic readiness checks, automatic review authority, or oracle-like inspection behavior.

## Further Notes

The official goals model changes the ownership boundary. A goal turns a Codex thread into a persisted objective with accounting and disciplined completion. Codex1 should not compete with that.

The target mental model is:

- Official Codex goals are the persisted thread objective and continuation engine.
- Codex1 is the durable artifact and evidence system.
- Codex remains the semantic judge.
- Codex1 never becomes a workflow oracle.
