# Codex1 PRD: Tiny Setup And Mission Scaffold Helper

## Problem Statement

Codex1 exists to help Codex keep serious repo work legible without becoming a second workflow engine. The useful part is narrow: install repo-local Codex1 guidance and create a safe mission folder layout.

Everything that requires judgment belongs to Codex and its skills. Native Codex goals own persistent objectives, continuation, pause/resume, usage accounting, budgets, and completion. Codex1 should not compete with that platform layer.

The user-facing mental model should be simple:

- Native `/goal` means persistent objective and continuation.
- Codex1 CLI means `setup` and `init`.
- Codex1 skills mean clarify, PRD synthesis, planning, execution lanes, review evidence, proof, and closeout.
- Codex remains the semantic judge.

## Solution

Codex1 is a small CLI plus repo-local skill bundle.

The CLI owns only:

- `codex1 setup`: materialize, status-check, disable, enable, uninstall, back up, restore, and diagnose repo-local managed guidance.
- `codex1 init`: create `.codex1/missions/<mission-id>/` with the standard path-safe folder tree.

The skills own the mission workflow:

- `$clarify` sharpens intent while questions are allowed.
- `$create-prd` synthesizes known context into `PRD.md`.
- `$plan` writes the execution plan, specs, subplans, ADRs, and `GOAL_BRIEF.md` when useful.
- Lane skills guide execution where they fit.
- `$codex-review` provides advisory review evidence.

`GOAL_BRIEF.md` helps Codex create or refine a native `/goal`; it is not goal state by itself.

## User Stories

1. As a Codex1 user, I want native `/goal` to own long-running continuation, so persistent objectives follow the official Codex model.
2. As a Codex1 user, I want setup to install repo-local workflow guidance, so Codex can discover the conventions in enabled repos.
3. As a Codex1 user, I want init to create the mission directory tree safely, so artifacts have a predictable home.
4. As a Codex1 user, I want PRDs, plans, specs, subplans, proofs, reviews, triage, and closeout to stay readable, so future sessions can understand the work.
5. As a Codex1 user, I want no CLI readiness oracle, so correctness and completion remain Codex judgments over actual evidence.
6. As a maintainer, I want the CLI command surface to stay tiny, so bloat does not creep back.
7. As a maintainer, I want setup backups and path containment well tested, so the real machinery remains reliable.
8. As a future Codex session, I want docs to say exactly where Codex1 stops, so I do not look for hidden state.

## Implementation Decisions

- Keep the public CLI surface to `setup` and `init`.
- Keep JSON envelopes stable for success and errors.
- Keep path safety as a deep module around contained writes and mission ID validation.
- Keep subplan lifecycle folders visible and file-based in the initialized layout.
- Keep goal brief creation as skill work, not a CLI trigger.
- Keep setup scoped to repo-local managed skills, docs, guidance, marker, and backups.
- Keep setup status mechanical; it reports bundle health only.
- Do not implement native goal APIs inside Codex1.

## Testing Decisions

Good tests assert external behavior:

- `init` creates the expected mission directories and rejects unsafe IDs or symlinked path components.
- Help output advertises only `init` and `setup`.
- Unknown commands fail through the normal argument parser.
- Setup materializes, reports, disables, enables, uninstalls, backs up, restores, and diagnoses repo-local managed guidance.
- Setup refuses to overwrite unmanaged files.
- Docs describe the small boundary.

## Out Of Scope

- Implementing native goals inside Codex1.
- Adding new execution state to the CLI.
- Making setup status into mission status.
- Adding semantic artifact authoring commands to the CLI.
- Redesigning the artifact model beyond the minimal setup/init boundary.

## Further Notes

The healthy shape is intentionally smaller than the original idea: Codex owns judgment and execution; Codex1 owns setup and safe scaffolding.
