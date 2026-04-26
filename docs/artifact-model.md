# Artifact Model

The artifact tree is the durable product. Files are human-facing markdown with minimal frontmatter and deterministic section tags.

## Mission Artifacts

`PRD.md` captures the mission goal, interpreted destination, success criteria, constraints, assumptions, proof expectations, review expectations, and PR intent. It is the anchor for durable work.

`PLAN.md` is the living strategy map. It describes workstreams, phases, risks, research posture, artifact links, review posture, and recommended slices. It is not a status dashboard or proof ledger.

`RESEARCH_PLAN.md` is optional. Codex writes it when research is substantial enough to need durable structure.

`CLOSEOUT.md` summarizes how Codex judges the PRD was satisfied, including completed, superseded, paused, or deferred work and remaining risks.

## Collection Artifacts

`RESEARCH/` stores research records: sources inspected, facts found, experiments run, uncertainties, options, recommendations, and affected artifacts.

`SPECS/` stores bounded implementation contracts. Specs describe responsibility, PRD relevance, scope, expected behavior, interfaces, proof expectations, and risks.

`SUBPLANS/` stores executable slices in visible lifecycle folders. Folder placement is a cue for humans and Codex, not a CLI state machine. Multiple files may be in `active/`.

`ADRS/` stores durable architecture decisions and tradeoffs.

`REVIEWS/` stores reviewer opinions. Reviews do not mutate mission truth.

`TRIAGE/` stores main-Codex adjudication of reviews. Triage explains accepted, rejected, deferred, duplicate, or stale findings.

`PROOFS/` stores evidence records for completed subplans: commands, tests, manual checks, changed areas, failures, accepted risks, and links.

## Machine Substrate

`.codex1/LOOP.json` is the explicit continuation loop state used by Ralph.

`.codex1/receipts/` stores optional audit receipts. Receipts are not replay authority.

`.codex1/events.jsonl` stores automatic forensic command metadata. Events help explain mechanical command history during debugging, but they are not durable content truth, not receipts, not workflow state, and not replay authority. Normal planning and execution should ignore events unless mission archaeology is needed.

There is no authoritative `STATE.json`.
