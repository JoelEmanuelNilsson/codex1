# Codex1 PRD: Mission-Local Forensic Events Log

## Problem Statement

Codex1 artifacts are durable mission truth, Codex is the semantic judge, and the CLI is a deterministic artifact helper. That boundary avoids the old failure mode where the CLI tried to become a smart status oracle.

The artifact workflow still benefits from a boring forensic trail of what the CLI itself did. When a mission contains several generated artifacts, moved subplans, receipts, or overwrite attempts, a future Codex session can inspect files but cannot easily answer mechanical questions such as:

- When was this mission initialized?
- Which command wrote this artifact?
- Was this subplan moved intentionally?
- Was a receipt appended through the CLI?
- Did a mutating command fail after resolving the mission?
- Did a weird sequence of commands happen before the artifacts got into this shape?

Most of the time this history will never be read. But when something looks strange, a small append-only forensic trail can make the difference between guessing and knowing.

The risk is that an event log could accidentally become the old `STATE.json` under a new name. If Codex1 starts deriving readiness, completion, review status, proof sufficiency, close safety, graph waves, native goal status, or next actions from events, the product has regressed. The event log must therefore be metadata-only, forensic-only, and non-authoritative.

## Solution

Add a mission-local `events.jsonl` file that records automatic, append-only metadata events for mutating Codex1 commands.

From the user's perspective, nothing new is required during normal use. Codex1 quietly records what it mechanically did in the background. If a mission later looks confusing, Codex or a human can inspect the event log to understand command history.

Each event line is JSON, versioned, timestamped, and deliberately small. Records describe command category, event kind, result, artifact kind when applicable, mission-relative paths, and other safe metadata. Records must not include raw argv, free-form answer payloads, receipt messages, command output, absolute filesystem paths, full artifact content, review finding text, or semantic claims about mission status.

The event log is best-effort. A successful artifact write, subplan move, or receipt append must not be rolled back or reported as failed merely because the forensic log could not be appended. In that case, the command succeeds and surfaces a warning. The log is observability, not authority.

Read-only commands remain read-only. Template commands, inspection, and doctor do not append events.

Inspection may report the count of event log entries and shallow mechanical warnings for malformed event lines. Inspection must not summarize last activity, infer progress, infer readiness, infer native goal status, or use event history to produce a status-like view.

The governing rule is:

> `events.jsonl` is security-camera footage. It is never mission truth.

## User Stories

1. As a user, I want Codex1 to record when a mission is initialized, so that the mission has a simple origin breadcrumb.
2. As a user, I want Codex1 to record when it writes an artifact, so that I can later see which CLI action created it.
3. As a user, I want Codex1 to record when a subplan is moved, so that lifecycle folder changes are auditable.
4. As a user, I want Codex1 to record when receipts are appended, so that intentional notes have a mechanical command trail.
5. As a user, I want mutating command failures to be logged after safe mission resolution, so that failed write attempts are visible when relevant.
6. As a user, I do not want unsafe mission ID or path failures to create logs anywhere, so that invalid paths cannot cause background writes.
7. As a user, I want read-only commands to stay read-only, so that inspecting or diagnosing a mission does not mutate it.
8. As a user, I want the event log to be mission-local, so that each mission's forensic trail travels with that mission.
9. As a user, I do not want a repo-global command diary, so that unrelated missions are not mixed together.
10. As a user, I want event entries to use mission-relative paths, so that logs are portable and do not leak my local machine layout.
11. As a user, I do not want raw command arguments stored, so that answer payloads, messages, and private paths do not leak into the log.
12. As a user, I do not want artifact content duplicated in the event log, so that artifacts remain the only durable content truth.
13. As a user, I do not want receipt messages duplicated in the event log, so that receipts remain intentional narrative records.
14. As a user, I want event entries to be timestamped, so that I can understand rough chronology.
15. As a user, I want JSONL file order to be the primary order, so that the log stays simple and append-only.
16. As a user, I do not want sequence numbers, so that Codex1 does not need a hidden counter or ledger system.
17. As Codex, I want artifact files to remain the source of content truth, so that I do not reconcile duplicate artifact payloads.
18. As Codex, I want to ignore event history during normal planning and execution, so that workflows remain artifact-centered.
19. As a future Codex session, I want to see command history without reading raw transcripts, so that mission archaeology is easier.
20. As a maintainer, I want event logging to be small and boring, so that it does not revive abandoned smart-state architecture.

## Implementation Decisions

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
- Omit receipt message text.
- Omit review finding text.
- Omit arbitrary user or Codex free-form prose.
- Log `mission_initialized` after mission initialization succeeds and the mission directory is safe.
- Log successful artifact writes with artifact kind, template version, overwrite flag, and mission-relative written path.
- Log artifact write failures only after a safe mission layout is resolved.
- Log successful subplan moves with subplan path, previous lifecycle folder, and new lifecycle folder.
- Log subplan move failures only after a safe mission layout is resolved.
- Log successful receipt appends with mission-relative receipt log path.
- Log receipt append failures only after a safe mission layout is resolved.
- Do not log template list or template show.
- Do not log inspect.
- Do not log doctor.
- If event append fails after a successful command, return command success with a warning.
- If event append fails while trying to log a failed mutating command, return the original command error.
- Inspection must not derive activity, readiness, progress, review status, close status, native goal status, or next action from events.
- Keep optional receipts separate from events.

## Testing Decisions

The main behavior tests should cover:

- Mission initialization appends a `mission_initialized` event.
- Artifact interview writes append `artifact_written` events.
- Artifact write events include artifact kind, template version, overwrite metadata, and mission-relative paths.
- Artifact write events do not include answers, raw argv, absolute paths, or generated markdown content.
- Subplan moves append `subplan_moved` events with old and new lifecycle folders.
- Receipt appends create receipt content and append a separate receipt event without duplicating the receipt message.
- Mutating command failures after safe mission resolution append failure events where feasible.
- Unsafe mission ID and path failures do not create event logs.
- Event append failure after a successful mutation returns success with a warning.
- Event append failure while logging a failed command does not hide the original command error.
- Read-only commands do not create or append event logs.
- Inspection reports event count.
- Inspection warns on malformed event log lines.
- Inspection does not emit semantic status fields derived from the event log.
- Malformed event logs do not block future event appends.
- Event records do not include raw command output, arbitrary free-form payloads, or absolute filesystem paths.

## Out Of Scope

- Semantic mission state.
- Native goal state.
- `STATE.json`.
- Event replay.
- Command replay.
- Event-sourced reconstruction of artifacts.
- Readiness computation.
- Completion computation.
- Review pass or fail computation.
- Close safety computation.
- Proof sufficiency computation.
- Graph or wave scheduling.
- Next-action projection.
- Global repo command log.
- Telemetry.
- Automatic upload or sync.
- Raw argv storage.
- Artifact content storage.
- Answer payload storage.
- Receipt message storage.
- Review finding text storage.
- Command output storage.
- Skill behavior that depends on event history during ordinary execution.

## Further Notes

The event log is valuable precisely because it is boring. It gives Codex and humans a forensic breadcrumb trail without giving the CLI new authority. The product should resist any future request to make events drive workflow decisions.

The recommended design mantra is:

> Artifacts are durable mission truth. Codex is the semantic judge. `events.jsonl` is just security-camera footage.
