# Codex1 Refactor Plan: Mission-Local Forensic Events Log

## Problem Statement

Codex1 has been rebuilt around the right product boundary: the CLI creates and moves durable artifacts, while Codex remains the semantic judge. That boundary is valuable because previous smart-status implementations repeatedly generated review findings around task readiness, review cleanliness, close safety, graph wave safety, terminal completion, stale proof, and replan priority.

The current implementation has one observability gap. When a mission's artifact tree looks odd, Codex and humans can inspect the files, but they cannot easily see the mechanical command history that produced them. There is no small mission-local trail showing that a mission was initialized, an artifact was written, a subplan was moved, or a receipt was appended.

The desired feature is a background forensic event log, not a state system. The log should help explain weird command history without influencing readiness, completion, review status, proof sufficiency, close safety, native goal status, or next action.

## Solution

Add a mission-local append-only `events.jsonl` file under each mission's internal metadata area.

The log records mutating Codex1 command outcomes as structured metadata:

- mission initialization;
- artifact writes;
- artifact write failures after safe mission resolution;
- subplan moves;
- subplan move failures after safe mission resolution;
- receipt appends;
- receipt append failures after safe mission resolution.

Read-only commands do not log. Inspection, template display, and doctor remain read-only.

The event log never stores raw argv, answer payloads, artifact body text, receipt message text, review finding text, command stdout/stderr, absolute paths, sequence numbers, replay checkpoints, or semantic status fields.

Event logging is best-effort. If the primary mutation succeeds but event append fails, the command still succeeds and surfaces a warning. If the primary mutation fails and the failure event cannot be appended, the original error remains the only command failure.

Inspection is extended only mechanically: it reports an event count and shallow warnings for malformed event lines. It does not show last activity, status, readiness, progress, close state, review state, native goal state, or any next action.

## Commits

1. **Add the event-log PRD to the repository.**
   Commit the feature PRD as a durable planning artifact. This commit does not change runtime behavior.

2. **Add an event-log location to the mission layout model.**
   Expose the mission-local event log path from the layout module. Keep the path inside the existing internal mission metadata directory.

3. **Add an event module skeleton.**
   Introduce a small module for forensic events. Add an empty public surface with no command integration yet. The codebase should compile with no behavior change.

4. **Define the event kind enum.**
   Add event kinds for mission initialization, artifact writes, artifact write failures, subplan moves, subplan move failures, receipt appends, and receipt append failures.

5. **Define the event result enum.**
   Add an explicit success/error result field. Keep it distinct from semantic mission state.

6. **Define safe event metadata.**
   Metadata may include artifact kind, template version, overwrite flag, mission-relative paths, lifecycle source/destination, receipt path, and error code. Metadata must not include free-form payload text.

7. **Define the event record envelope.**
   Add a versioned event record with timestamp, mission id, command name, event kind, result, duration in milliseconds when supplied, and command-family metadata. Do not add sequence numbers, event IDs, raw argv, cwd, stdout, or stderr.

8. **Add mission-relative path rendering for events.**
   Implement a helper that converts mission-contained paths into mission-relative strings. Reject or omit paths that cannot be proven inside the mission.

9. **Add append-only event writing.**
   Implement best-effort JSONL append using the existing path containment helpers. Create the parent metadata directory when needed. Reject symlinked event file targets or escaping parents through existing path-safety rules.

10. **Make event append failure non-fatal.**
    Return a warning object from the event append helper instead of converting append failure into a command failure.

11. **Add shallow event log scanning.**
    Implement a helper that counts parseable event lines and returns mechanical warnings for malformed JSON, unsupported version, missing event kind, or non-object rows.

12. **Add warning support to JSON and human output.**
    JSON success envelopes may include a top-level warnings array. Human mode prints warnings to stderr.

13. **Log mission initialization success.**
    After mission directory creation succeeds, append a `mission_initialized` event.

14. **Prove failed initialization does not log before safe layout exists.**
    Add a regression test for unsafe mission id or path initialization failure.

15. **Log artifact write success and failure.**
    Append `artifact_written` after successful writes and `artifact_write_failed` after safe-layout failures.

16. **Assert artifact write events do not leak payloads.**
    Add a regression test with distinctive answer text and an answers-file path.

17. **Log subplan move success and failure.**
    Append `subplan_moved` after moves and `subplan_move_failed` after safe-layout failures.

18. **Log receipt append success and failure.**
    Append `receipt_appended` after writes and `receipt_append_failed` after safe-layout failures. Do not duplicate the receipt message.

19. **Prove read-only commands do not log.**
    Add integration tests showing template list, template show, inspect, and doctor do not create or append an event log.

20. **Extend inspect inventory with event count and shallow warnings.**
    Report event count and malformed-line warnings without interpreting event history.

21. **Add anti-oracle regression coverage for events.**
    Assert inspect output does not contain fields such as next action, ready, complete, blocked, review passed, close ready, replan required, task status, last event timestamp, activity status, progress, or native goal state.

22. **Prove malformed logs do not block future appends.**
    Create a malformed event log, run a mutating command, and assert a new valid event is appended after the malformed line.

23. **Document the event log in the CLI contract, artifact model, and README.**
    Explain path, mutating-command coverage, metadata-only schema, warning behavior, read-only exclusions, and the anti-oracle rule.

24. **Run formatting and the full test suite.**
    Run formatting, unit tests, integration tests, clippy with warnings denied where practical, and any existing compile checks.

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
- Native Codex goals remain outside Codex1.
- Events are not used to derive readiness, completion, review status, proof sufficiency, close safety, graph waves, replan priority, native goal status, or next action.
- Read-only commands do not log.
- Mutating command successes log after the primary mutation succeeds.
- Failed mutating commands may log only after safe mission layout resolution.
- Unsafe mission ID and unsafe path failures do not log.
- Event append failure never rolls back or fails the primary successful command.
- Event records use JSONL line order instead of sequence numbers.
- Event records include a version field, ISO timestamp, optional duration, command summary metadata, and mission-relative paths.
- Event records do not include absolute paths, free-form answer text, artifact content, receipt message text, review finding text, stdout, stderr, or raw argv.
- Receipts remain separate from events.
- Inspection may count event entries and shallow-warn on malformed event lines.
- Inspection does not show last event timestamp or interpret event history.

## Testing Decisions

The highest-priority tests are regression tests for the product boundary:

- no raw argv leakage;
- no answer payload leakage;
- no artifact content duplication;
- no receipt message leakage;
- no absolute path leakage;
- no read-only command logging;
- no semantic status fields in inspect;
- no event append failure breaking real work.

The next priority is event coverage for mutating commands:

- init;
- interview artifact writes;
- singleton collision or equivalent write failure after safe mission resolution;
- subplan move success and failure;
- receipt append success and failure where practical.

Mechanical validation tests should cover:

- valid JSONL line counting;
- malformed JSONL warnings;
- non-object row warnings;
- unsupported version warnings;
- missing event kind warnings;
- malformed logs not blocking future appends.

## Out Of Scope

- Semantic mission state.
- Native goal state.
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
- Receipt message capture.
- Review finding capture.
- Skills using event history during ordinary execution.

## Further Notes

This feature is worthwhile only if it stays modest. The right implementation should feel like a compact forensic helper, not a new runtime.

The main failure mode to avoid is treating event history as a source of current truth. The only valid questions are mechanical and forensic: "what command metadata was appended?" and "is the log itself parseable enough to count?"
