# Codex1 PRD: Mission-Local Forensic Events Log

## Problem Statement

Codex1 now has the right authority boundary: artifacts are durable mission truth, Codex is the semantic judge, and the CLI is a deterministic artifact helper. That solved the earlier failure mode where the CLI tried to become a smart status oracle.

However, the current artifact workflow still lacks a boring forensic trail of what the CLI itself did. When a mission contains several generated artifacts, moved subplans, loop changes, receipts, or overwrite attempts, a future Codex session can inspect the files but cannot easily answer mechanical questions such as:

- When was this mission initialized?
- Which command wrote this artifact?
- Was this subplan moved intentionally?
- Did a loop get paused or stopped through the CLI?
- Did a mutating command fail after resolving the mission?
- Did a weird sequence of commands happen before the artifacts got into this shape?

Most of the time this history will never be read. But when something looks strange, a small append-only forensic trail can make the difference between guessing and knowing.

The risk is that an event log could accidentally become the old `STATE.json` under a new name. If Codex1 starts deriving readiness, completion, review status, proof sufficiency, close safety, graph waves, or next actions from events, the product has regressed. The event log must therefore be explicitly metadata-only, forensic-only, and non-authoritative.

## Solution

Add a mission-local `events.jsonl` file that records automatic, append-only metadata events for mutating Codex1 commands.

From the user's perspective, nothing new is required during normal use. Codex1 quietly records what it mechanically did in the background. If a mission later looks confusing, Codex or a human can inspect the event log to understand command history.

The log lives inside each mission's internal metadata area. It travels with the mission, does not mix unrelated missions, and can be deleted with the mission. It is not repo-global and not a separate database.

Each event line is JSON, versioned, timestamped, and deliberately small. Records describe command category, event kind, result, artifact kind when applicable, mission-relative paths, and other safe metadata. Records must not include raw argv, free-form answer payloads, loop messages, receipt messages, command output, absolute filesystem paths, full artifact content, review finding text, or semantic claims about mission status.

The event log is best-effort. A successful artifact write, subplan move, receipt append, or loop state change must not be rolled back or reported as failed merely because the forensic log could not be appended. In that case, the command succeeds and surfaces a warning. The log is observability, not authority.

Read-only commands remain read-only. Template commands, inspection, doctor, and Ralph Stop-hook invocations do not append events. Ralph must stay especially boring: it reads explicit loop state and may block, but it does not mutate mission files.

Inspection may report the count of event log entries and shallow mechanical warnings for malformed event lines. Inspection must not summarize last activity, infer progress, infer readiness, or use event history to produce a status-like view.

The governing rule is:

> `events.jsonl` is security-camera footage. It is never mission truth.

## User Stories

1. As a user, I want Codex1 to record when a mission is initialized, so that the mission has a simple origin breadcrumb.
2. As a user, I want Codex1 to record when it writes an artifact, so that I can later see which CLI action created it.
3. As a user, I want Codex1 to record when a subplan is moved, so that lifecycle folder changes are auditable.
4. As a user, I want Codex1 to record when loop state changes, so that Ralph-related behavior is easier to understand later.
5. As a user, I want Codex1 to record when receipts are appended, so that intentional notes have a mechanical command trail.
6. As a user, I want mutating command failures to be logged after safe mission resolution, so that failed write attempts are visible when relevant.
7. As a user, I do not want unsafe mission ID or path failures to create logs anywhere, so that invalid paths cannot cause background writes.
8. As a user, I want read-only commands to stay read-only, so that inspecting or diagnosing a mission does not mutate it.
9. As a user, I want Ralph Stop-hook invocations to stay read-only, so that Stop hooks do not create noisy or surprising writes.
10. As a user, I want the event log to be mission-local, so that each mission's forensic trail travels with that mission.
11. As a user, I do not want a repo-global command diary, so that unrelated missions are not mixed together.
12. As a user, I want event entries to use mission-relative paths, so that logs are portable and do not leak my local machine layout.
13. As a user, I do not want raw command arguments stored, so that answer payloads, messages, and private paths do not leak into the log.
14. As a user, I do not want artifact content duplicated in the event log, so that artifacts remain the only durable content truth.
15. As a user, I do not want loop messages duplicated in the event log, so that loop state remains the place where loop message text lives.
16. As a user, I do not want receipt messages duplicated in the event log, so that receipts remain intentional narrative records.
17. As a user, I want event entries to be timestamped, so that I can understand rough chronology.
18. As a user, I want JSONL file order to be the primary order, so that the log stays simple and append-only.
19. As a user, I do not want sequence numbers, so that Codex1 does not need a hidden counter or ledger system.
20. As a user, I want a small fixed set of event kinds, so that the log is predictable without becoming a schema bureaucracy.
21. As a user, I want event records to include command duration when cheap, so that slow or strange commands can be diagnosed.
22. As a user, I want event append failures to warn rather than fail the command, so that forensic logging cannot hold real work hostage.
23. As a user, I want a JSON warning when event logging fails under JSON mode, so that machine callers can notice degraded audit behavior.
24. As a user, I want a stderr warning when event logging fails under human mode, so that the issue is visible without changing the command result.
25. As a user, I want failed mutating commands to return their original error even if failure logging also fails, so that the real failure is not hidden.
26. As a user, I want inspection to count event entries, so that I can see whether a forensic trail exists.
27. As a user, I want inspection to shallow-check malformed event lines, so that corrupted logs are mechanically visible.
28. As a user, I do not want inspection to show last event timestamp, so that inspect does not start feeling like activity status.
29. As a user, I do not want inspection to infer progress from events, so that the CLI does not become a status oracle again.
30. As a user, I want malformed event logs not to block future commands, so that a damaged forensic trail does not break artifact work.
31. As a user, I want event records to include an event schema version, so that future changes can be handled explicitly.
32. As a user, I want event records to include artifact kind for artifact writes, so that the history is understandable without reading paths alone.
33. As a user, I want event records to include template version for generated artifacts, so that template-generated content can be traced.
34. As a user, I want event records to include old and new lifecycle folders for subplan moves, so that movement is clear.
35. As a user, I want event records to avoid arbitrary free-form metadata, so that secrets and duplicate truth do not sneak in.
36. As a user, I want the implementation to avoid storing sensitive values by design, so that redaction is not the primary safety mechanism.
37. As a user, I want obvious future sensitive metadata keys to be defensively redacted, so that accidental leakage is still reduced.
38. As Codex, I want a simple event log to debug weird mission shapes, so that I can reason about what happened without inventing a state model.
39. As Codex, I want artifact files to remain the source of content truth, so that I do not reconcile duplicate artifact payloads.
40. As Codex, I want to ignore event history during normal planning and execution, so that workflows remain artifact-centered.
41. As Codex, I want to consult event history only for debugging, so that the log remains a forensic tool.
42. As a future Codex session, I want to see command history without reading raw transcripts, so that mission archaeology is easier.
43. As a maintainer, I want event logging to be small and boring, so that it does not revive the abandoned smart-state architecture.
44. As a maintainer, I want event logging to use a deep, isolated module, so that append behavior can be tested without coupling to command logic.
45. As a maintainer, I want event logging integration to be explicit at mutation points, so that read-only commands cannot accidentally log.
46. As a maintainer, I want event append failures to be non-fatal by contract, so that old transaction bugs do not return.
47. As a maintainer, I want shallow inspect validation for event logs, so that corrupted logs produce warnings without semantic interpretation.
48. As a maintainer, I want regression tests proving no raw argv or payload text is logged, so that privacy boundaries stay intact.
49. As a maintainer, I want regression tests proving Ralph does not log, so that Stop-hook behavior stays fail-open and read-only.
50. As a tester, I want tests for every mutating event type, so that the forensic trail is not accidentally partial.
51. As a tester, I want tests for event append failure warnings, so that warning behavior remains stable.
52. As a tester, I want tests for malformed event log inspection warnings, so that inspect stays mechanically useful.
53. As a tester, I want tests proving event logs do not include semantic status fields, so that oracle behavior does not creep in.
54. As a tester, I want tests proving events use mission-relative paths, so that absolute path leakage is prevented.
55. As a tester, I want tests proving read-only commands do not create event logs, so that read-only commands remain pure reads.

## Implementation Decisions

- Add a small event logging module that owns event record construction, mission-relative path rendering, metadata normalization, defensive redaction, JSONL serialization, append behavior, and shallow log validation.
- Add a fixed versioned event record shape.
- Store the log inside each mission's internal metadata area as `events.jsonl`.
- Treat the event log as append-only.
- Do not implement rotation, truncation, compaction, replay, checkpoints, sequence numbers, or global indexes.
- Use JSONL line order as the canonical order.
- Add an ISO timestamp to each event record.
- Add command duration in milliseconds when cheap to collect.
- Store command summaries, not raw argv.
- Store event metadata, not artifact content.
- Store mission-relative paths only.
- Omit absolute paths.
- Omit command stdout and stderr.
- Omit answer file contents.
- Omit loop message text.
- Omit receipt message text.
- Omit review finding text.
- Omit arbitrary user or Codex free-form prose.
- Prefer schema design that has nowhere sensitive text belongs.
- Add minimal defensive redaction for obvious future sensitive metadata key names.
- Log `mission_initialized` after mission initialization succeeds and the mission directory is safe.
- Do not log failed initialization before a safe mission layout exists.
- Log successful artifact writes with artifact kind, template version, overwrite flag, and mission-relative written path.
- Log artifact write failures only after a safe mission layout is resolved.
- Log successful subplan moves with subplan path, previous lifecycle folder, and new lifecycle folder.
- Log subplan move failures only after a safe mission layout is resolved.
- Log successful receipt appends with mission-relative receipt log path.
- Log receipt append failures only after a safe mission layout is resolved.
- Log successful loop start, pause, resume, and stop events.
- Log loop mutation failures only after a safe mission layout is resolved.
- Do not log template list or template show.
- Do not log inspect.
- Do not log doctor.
- Do not log Ralph Stop-hook.
- Do not log read-only failures.
- If event append fails after a successful command, return command success with a warning.
- If event append fails while trying to log a failed mutating command, return the original command error.
- Surface event logging warnings in JSON envelopes when JSON mode is active.
- Surface event logging warnings on stderr in human mode.
- Extend inspection with an event entry count.
- Extend inspection with shallow mechanical warnings for malformed JSONL lines, unsupported event versions, and missing event kind.
- Inspection must not show last event timestamp.
- Inspection must not derive activity, readiness, progress, review status, close status, or next action from events.
- Malformed event logs must not prevent future appends.
- Keep optional receipts separate from events.
- Receipts remain intentional narrative breadcrumbs.
- Events remain automatic command metadata.
- Document the forensic-only boundary in CLI contract documentation.
- Document that skills should normally ignore events and consult them only for debugging or archaeology.

The implementation should touch these conceptual modules:

- Mission layout: expose the internal event log location.
- Event logging: define event record types and append/validation helpers.
- Command execution: integrate best-effort logging at mutating command boundaries.
- JSON/human output: carry warnings without converting successful commands into failures.
- Inspection: count and shallow-validate event log lines.
- Tests: cover event generation, warning behavior, privacy boundaries, read-only purity, and anti-oracle regression.

## Testing Decisions

Good tests should verify externally observable behavior through the CLI and through isolated event-log helpers. They should not assert incidental implementation details such as private function names or internal formatting that is not part of the contract.

The main behavior tests should cover:

- Mission initialization appends a `mission_initialized` event.
- Artifact interview writes append `artifact_written` events.
- Artifact write events include artifact kind, template version, overwrite metadata, and mission-relative paths.
- Artifact write events do not include answers, raw argv, absolute paths, or generated markdown content.
- Subplan moves append `subplan_moved` events with old and new lifecycle folders.
- Receipt appends create receipt content and append a separate receipt event without duplicating the receipt message.
- Loop start, pause, resume, and stop append matching loop events without duplicating loop message text.
- Mutating command failures after safe mission resolution append failure events where feasible.
- Unsafe mission ID and path failures do not create event logs.
- Event append failure after a successful mutation returns success with a warning.
- Event append failure while logging a failed command does not hide the original command error.
- Read-only commands do not create or append event logs.
- Ralph Stop-hook does not create or append event logs.
- Inspection reports event count.
- Inspection warns on malformed event log lines.
- Inspection does not emit semantic status fields derived from the event log.
- Malformed event logs do not block future event appends.
- Event records use JSONL and can be parsed line by line.
- Event records include version and timestamp.
- Event records do not include sequence numbers.
- Event records do not include raw command output.
- Event records do not include arbitrary free-form payloads.
- Event records do not include absolute filesystem paths.
- Event logging remains best-effort and does not roll back artifact writes.

Prior art in the codebase:

- Existing CLI integration tests already verify JSON envelopes, path safety, mission initialization, artifact interviews, subplan lifecycle moves, inspect behavior, loop state, and Ralph Stop-hook behavior.
- Existing anti-oracle tests already ensure inspect does not emit status-like fields.
- Existing symlink and containment tests provide patterns for event log path-safety tests.
- Existing answer-file tests provide patterns for asserting payload data is not leaked into event records.

## Out of Scope

- No semantic mission state.
- No `STATE.json`.
- No event replay.
- No command replay.
- No event-sourced reconstruction of artifacts.
- No readiness computation.
- No completion computation.
- No review pass or fail computation.
- No close safety computation.
- No proof sufficiency computation.
- No graph or wave scheduling.
- No next-action projection.
- No global repo command log.
- No telemetry.
- No automatic upload or sync.
- No event rotation.
- No event compaction.
- No event sequence numbers.
- No event IDs unless a future use case proves they are needed.
- No raw argv storage.
- No artifact content storage.
- No answer payload storage.
- No loop message storage.
- No receipt message storage.
- No review finding text storage.
- No command output storage.
- No PRD or plan interpretation from events.
- No skill behavior that depends on event history during ordinary execution.

## Further Notes

The event log is valuable precisely because it is boring. It gives Codex and humans a forensic breadcrumb trail without giving the CLI new authority. The product should resist any future request to make events drive workflow decisions.

The recommended design mantra is:

> Artifacts are durable mission truth. Codex is the semantic judge. `events.jsonl` is just security-camera footage.

This feature should be small. A healthy implementation should feel like a compact helper module plus command integration and tests. If it starts growing into hundreds of lines of status logic, recovery logic, replay behavior, or semantic interpretation, the implementation is drifting back toward the old failure mode.
