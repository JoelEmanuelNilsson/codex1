# OUTCOME.md required fields

One line of intent per field. Full contract and bad-vs-good examples: `docs/codex1-rebuild-handoff/03-planning-artifacts.md`.

| Field | Intent |
| --- | --- |
| `mission_id` | Directory name under `PLANS/`. Stable, lowercase-hyphen. |
| `status` | `draft` until ratified; CLI flips to `ratified`. |
| `title` | Short human-readable mission title. |
| `original_user_goal` | Verbatim user request that started the mission. |
| `interpreted_destination` | Concrete end state another Codex thread can aim at without chat context. |
| `must_be_true` | Invariants that hold when the mission is done. |
| `success_criteria` | Testable, concrete conditions. No "works well" style. |
| `non_goals` | What is explicitly out of scope. |
| `constraints` | Explicit limits (tech, process, approval, time, cost). |
| `definitions` | Terms defined to remove ambiguity (map of term to meaning). |
| `quality_bar` | Specific quality expectations (coverage, style, perf, UX). |
| `proof_expectations` | What evidence counts as "task done" for the mission's tasks. |
| `review_expectations` | Review profiles expected (bug, intent, integration, security, etc.). |
| `known_risks` | Hazards foreseen at clarify time. |
| `resolved_questions` | List of `{ question, answer }` — includes inferences the clarifier recorded without asking. |
