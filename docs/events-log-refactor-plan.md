# Codex1 Refactor Plan: Mission-Local Forensic Events Log

## Problem Statement

Codex1 has been rebuilt around the right product boundary: the CLI creates and moves durable artifacts, while Codex remains the semantic judge. That boundary is valuable because previous smart-status implementations repeatedly generated review findings around task readiness, review cleanliness, close safety, graph wave safety, terminal completion, stale proof, and replan priority.

The current implementation still has one observability gap. When a mission's artifact tree looks odd, Codex and humans can inspect the files, but they cannot easily see the mechanical command history that produced them. There is no small mission-local trail showing that a mission was initialized, an artifact was written, a subplan was moved, a receipt was appended, or loop state was changed.

The desired feature is a background forensic event log, not a state system. The log should help explain weird command history without influencing readiness, completion, review status, proof sufficiency, close safety, or next action. The implementation must stay boring and small.

## Solution

Add a mission-local append-only `events.jsonl` file under each mission's internal metadata area.

The log records mutating Codex1 command outcomes as structured metadata:

- mission initialization;
- artifact writes;
- artifact write failures after safe mission resolution;
- subplan moves;
- subplan move failures after safe mission resolution;
- receipt appends;
- receipt append failures after safe mission resolution;
- loop start, pause, resume, and stop;
- loop mutation failures after safe mission resolution.

Read-only commands do not log. Inspection, template display, doctor, and Ralph Stop-hook remain read-only.

The event log never stores raw argv, answer payloads, artifact body text, loop message text, receipt message text, review finding text, command stdout/stderr, absolute paths, sequence numbers, replay checkpoints, or semantic status fields.

Event logging is best-effort. If the primary mutation succeeds but event append fails, the command still succeeds and surfaces a warning. If the primary mutation fails and the failure event cannot be appended, the original error remains the only command failure.

Inspection is extended only mechanically: it reports an event count and shallow warnings for malformed event lines. It does not show last activity, status, readiness, progress, close state, review state, or any next action.

## Commits

1. **Add the event-log PRD to the repository.**  
   Commit the feature PRD as a durable planning artifact. This commit does not change runtime behavior.

2. **Add an event-log location to the mission layout model.**  
   Expose the mission-local event log path from the layout module. Keep the path inside the existing internal mission metadata directory. Add a focused unit or integration assertion that the rendered location is mission-contained.

3. **Add an event module skeleton.**  
   Introduce a small module for forensic events. Add an empty public surface with no command integration yet. The codebase should compile with no behavior change.

4. **Define the event kind enum.**  
   Add a fixed set of event kinds for mission initialization, artifact writes, artifact write failures, subplan moves, subplan move failures, receipt appends, receipt append failures, loop starts, loop start failures, loop pauses, loop pause failures, loop resumes, loop resume failures, loop stops, and loop stop failures. Add serialization tests for stable kebab-case or snake-case names, whichever matches the current JSON style best.

5. **Define the event result enum.**  
   Add an explicit success/error result field. Keep it distinct from semantic mission state. Add tests that only the allowed result values serialize.

6. **Define safe event metadata structs.**  
   Add narrow metadata shapes for each command family. Metadata may include artifact kind, template version, overwrite flag, mission-relative paths, lifecycle source/destination, loop mode, message-present boolean, receipt-path, and error code. Metadata must not include free-form payload text. Add serialization tests.

7. **Define the event record envelope.**  
   Add a versioned event record with timestamp, mission id, command name, event kind, result, duration in milliseconds when supplied, and command-family metadata. Do not add sequence numbers, event IDs, raw argv, cwd, stdout, or stderr. Add serialization tests.

8. **Add mission-relative path rendering for events.**  
   Implement a helper that converts mission-contained paths into mission-relative strings. Reject or omit paths that cannot be proven inside the mission. Add tests for normal paths, nested paths, absolute source paths, and escape attempts.

9. **Add defensive metadata redaction.**  
   Add a tiny redaction helper for future string metadata keys containing obvious sensitive words such as password, secret, token, or key. Keep the schema designed to avoid sensitive strings in the first place. Add tests that redaction exists without relying on it for normal fields.

10. **Add append-only event writing.**  
   Implement best-effort JSONL append using the existing path containment helpers. Create the parent metadata directory when needed. Reject symlinked event file targets or escaping parents through existing path-safety rules. Add isolated tests for writing one event and multiple events.

11. **Make event append failure non-fatal at the event-module boundary.**  
   Return a warning object or warning string from the event append helper instead of converting append failure into a command failure. Add tests using a directory or symlink at the event log path to force append failure.

12. **Add shallow event log scanning.**  
   Implement a helper that counts parseable event lines and returns mechanical warnings for malformed JSON, unsupported version, missing event kind, or non-object rows. Do not validate semantic consistency. Add unit tests with mixed good and bad lines.

13. **Add warning support to JSON success envelopes.**  
   Extend success output to optionally include a top-level warnings array. Keep the existing `ok` and `data` fields stable. Add tests for success without warnings and success with warnings.

14. **Add warning support to human output.**  
   Add a small helper that prints warning messages to stderr for non-JSON commands. Keep normal stdout messages unchanged. Add a focused test where practical; otherwise cover through integration tests after event logging is wired.

15. **Log mission initialization success.**  
   After mission directory creation succeeds, append a `mission_initialized` event. If event append fails, return init success with a warning. Add integration tests for event creation and warning behavior.

16. **Prove failed initialization does not log before safe layout exists.**  
   Add a regression test for unsafe mission id or path initialization failure. Assert that no mission event log is created anywhere as a side effect.

17. **Log artifact write success.**  
   After an interview command writes an artifact, append an `artifact_written` event with artifact kind, template version, overwrite flag, and mission-relative artifact path. Add tests for PRD and one collection artifact.

18. **Assert artifact write events do not leak payloads.**  
   Add a regression test with distinctive answer text and an answers-file path. Assert the event log does not contain answer text, generated markdown body text, raw argv, or the absolute answers-file path.

19. **Log artifact write failures after safe mission resolution.**  
   For failures that occur after the mission layout is safely resolved, append `artifact_write_failed` with command metadata and error code where feasible. Do not log parse/path failures before safe mission resolution. Add tests for singleton collision and for an unsafe mission id.

20. **Log subplan move success.**  
   After a subplan move succeeds, append a `subplan_moved` event with mission-relative source and target paths plus old and new lifecycle folders. Add an integration test.

21. **Log subplan move failures after safe mission resolution.**  
   Add `subplan_move_failed` for safe-layout failures such as unknown subplan or duplicate target. Preserve the original command error if failure logging fails. Add tests.

22. **Log receipt append success.**  
   After a receipt append succeeds, append `receipt_appended` with the mission-relative receipt log path. Do not duplicate the receipt message. Add a test with distinctive receipt text proving it is absent from the event log.

23. **Log receipt append failures after safe mission resolution.**  
   Add `receipt_append_failed` where feasible. Keep failure logging best-effort and non-masking. Add tests with an invalid receipt target shape if practical.

24. **Log loop start success.**  
   After loop start writes explicit loop state, append `loop_started` with mode and message-present boolean. Do not include the loop message text. Add tests.

25. **Log loop pause, resume, and stop success.**  
   Append `loop_paused`, `loop_resumed`, and `loop_stopped` after successful loop mutations. For pause and stop, record reason-present boolean only, not reason text. Add tests.

26. **Log loop mutation failures after safe mission resolution.**  
   Add failure events for loop start, pause, resume, and stop when the mission layout is safe and the mutation fails. Add tests for missing loop state on pause/resume/stop if that is the current failure behavior.

27. **Prove read-only commands do not log.**  
   Add integration tests showing template list, template show, inspect, doctor, and Ralph Stop-hook do not create or append an event log.

28. **Prove Ralph stays read-only under block and allow paths.**  
   Add tests for Ralph allowing and blocking while asserting event log contents do not change. This specifically protects the Stop-hook from becoming noisy or stateful.

29. **Extend inspect inventory with event count.**  
   Add an event count to the artifact inventory. This is mechanical inventory only. Add tests for no log, empty log, and multiple-line log.

30. **Extend inspect with shallow event warnings.**  
   Surface warnings for malformed event log rows using the existing mechanical warning pattern. Add tests for malformed JSON, non-object rows, unsupported versions, and missing event kind.

31. **Add anti-oracle regression coverage for events.**  
   Extend or add tests asserting inspect output still does not contain fields such as next action, ready, complete, blocked, review passed, close ready, replan required, task status, last event timestamp, activity status, or progress.

32. **Prove malformed logs do not block future appends.**  
   Create a malformed event log, run a mutating command, and assert a new valid event is appended after the malformed line. Inspect should warn but the mutation should succeed.

33. **Prove append failure warns but does not fail primary mutation.**  
   Make the event log path unwritable or structurally invalid after mission creation. Run a mutating command whose primary write can still succeed. Assert command success, warning emission, and primary artifact/loop change.

34. **Verify warning shape in JSON mode.**  
   Add an integration test asserting a successful command with event append failure returns `ok: true`, includes `data`, and includes a warnings array. Do not change error envelope shape.

35. **Verify warning shape in human mode.**  
   Add an integration test asserting the command exits successfully, writes normal stdout, and emits a concise warning on stderr when event append fails.

36. **Document the event log in the CLI contract.**  
   Add a concise section explaining path, mutating-command coverage, metadata-only schema, warning behavior, read-only exclusions, and the anti-oracle rule.

37. **Document the event log in the artifact model.**  
   Explain that events are internal forensic command metadata, not durable content truth, not receipts, and not workflow state.

38. **Update the README quickstart or concepts section.**  
   Mention that Codex1 keeps a mission-local forensic event log automatically, usually ignored unless debugging weird mission history.

39. **Run formatting and the full test suite.**  
   Run formatting, unit tests, integration tests, clippy with warnings denied, and any existing compile checks. Fix any issues without expanding scope.

40. **Final anti-bloat review.**  
   Read the implementation and remove any accidental semantic status logic, replay logic, sequence allocation, global log behavior, raw payload capture, or command-history interpretation before considering the feature done.

## Decision Document

- The event log is named `events.jsonl`.
- The event log is mission-local.
- The event log lives inside each mission's internal metadata area.
- The event log is append-only.
- The event log is metadata-only.
- The event log is forensic-only.
- The event log is never mission truth.
- Artifacts remain durable mission truth.
- Codex remains the semantic judge.
- Events are not used to derive readiness, completion, review status, proof sufficiency, close safety, graph waves, replan priority, or next action.
- Read-only commands do not log.
- Ralph Stop-hook does not log.
- Template display does not log.
- Doctor does not log.
- Inspect does not log.
- Mutating command successes log after the primary mutation succeeds.
- Failed mutating commands may log only after safe mission layout resolution.
- Unsafe mission ID and unsafe path failures do not log.
- Event append failure never rolls back or fails the primary successful command.
- JSON success envelopes may include warnings.
- Human-mode warnings go to stderr.
- Failure-event append failure never hides the original command error.
- Event records use JSONL line order instead of sequence numbers.
- Event records include a version field.
- Event records include an ISO timestamp.
- Event records may include command duration in milliseconds.
- Event records include command summary metadata rather than raw argv.
- Event records include mission-relative paths only.
- Event records do not include absolute paths.
- Event records do not include free-form answer text.
- Event records do not include artifact content.
- Event records do not include loop message text.
- Event records do not include receipt message text.
- Event records do not include review finding text.
- Event records do not include command stdout or stderr.
- Event metadata is represented as a small fixed union, not a loose arbitrary object.
- Defensive redaction exists only as a backstop for future metadata mistakes.
- Receipts remain separate from events.
- Receipts are intentional narrative breadcrumbs.
- Events are automatic command metadata.
- Inspection may count event entries.
- Inspection may shallow-warn on malformed event lines.
- Inspection does not show last event timestamp.
- Inspection does not interpret event history.
- Malformed event logs do not block future event appends.
- The implementation should be small enough to remain obviously non-oracular.

## Testing Decisions

Good tests should exercise the public CLI behavior and the event module's stable helper behavior. They should not test private implementation details that can change without altering the contract.

The highest-priority tests are regression tests for the product boundary:

- no raw argv leakage;
- no answer payload leakage;
- no artifact content duplication;
- no loop message leakage;
- no receipt message leakage;
- no absolute path leakage;
- no read-only command logging;
- no Ralph logging;
- no semantic status fields in inspect;
- no event append failure breaking real work.

The next priority is event coverage for mutating commands:

- init;
- interview artifact writes;
- singleton collision or equivalent write failure after safe mission resolution;
- subplan move success and failure;
- receipt append success and failure where practical;
- loop start, pause, resume, and stop success;
- loop mutation failures where practical.

Mechanical validation tests should cover:

- valid JSONL line counting;
- malformed JSONL warnings;
- non-object row warnings;
- unsupported version warnings;
- missing event kind warnings;
- malformed logs not blocking future appends.

Existing tests provide prior art for:

- JSON envelope assertions;
- mission path safety;
- symlink rejection;
- artifact interview output;
- subplan lifecycle movement;
- inspect anti-oracle behavior;
- loop state behavior;
- Ralph Stop-hook behavior;
- answer-file validation and payload control.

The final verification set should include:

- full integration test suite;
- unit tests;
- formatting;
- clippy with warnings denied;
- a quick manual smoke of init, PRD interview, inspect, loop start, and Ralph Stop-hook if time permits.

## Out of Scope

- Semantic mission state.
- `STATE.json`.
- Event replay.
- Command replay.
- Reconstructing artifacts from events.
- Sequence numbers.
- Event IDs.
- Global repo logs.
- Telemetry.
- Uploading or syncing event logs.
- Log rotation.
- Log compaction.
- Event search.
- Status projection.
- Next-action projection.
- Task readiness.
- Review pass/fail.
- Proof sufficiency.
- Close safety.
- PRD satisfaction.
- Graph wave computation.
- Replan priority.
- Raw argv capture.
- Full command transcript capture.
- Artifact content capture.
- Answer payload capture.
- Loop message capture.
- Receipt message capture.
- Review finding capture.
- Skills using event history during ordinary execution.

## Further Notes

This feature is worthwhile only if it stays modest. The right implementation should feel like a compact forensic helper, not a new runtime.

The main failure mode to avoid is treating event history as a source of current truth. If the implementation starts asking "what does the event log imply about the mission?" it has gone wrong. The only valid questions are mechanical and forensic: "what command metadata was appended?" and "is the log itself parseable enough to count?"

The product mantra remains:

> Artifacts are durable mission truth. Codex is the semantic judge. `events.jsonl` is just security-camera footage.
