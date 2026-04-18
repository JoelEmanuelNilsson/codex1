---
artifact: mission-state
mission_id: global-codex1-installer
root_mission_id: global-codex1-installer
parent_mission_id: null
version: 1
clarify_status: ratified
slug: global-codex1-installer
current_lock_revision: 1
reopened_from_lock_revision: null
---

# Mission State

## Objective Snapshot

- Mission title: Global Codex1 Installer
- Current interpreted objective: Make Codex1 install and verify as a general machine-level Codex runtime add-on, inspired by oh-my-codex, with backups, doctor checks, and a clean split: `codex1 setup` configures the user's machine and Codex runtime, while `codex1 init` explicitly configures the current project.
- Current phase hint: clarify complete; ready for `$plan`

## Ambiguity Register

| Dimension | Score (0-3) | Why it still matters | Planned reducer | Provenance |
| --- | --- | --- | --- | --- |
| Objective clarity | 0 | Destination is locked around installer-style global setup plus explicit project init. | Handoff to plan. | user ratification |
| Success proof | 1 | Exact test commands and fixture design belong in planning, but command-level proof bar is locked. | Plan proof matrix. | clarify synthesis |
| Protected surfaces | 1 | Protected surfaces are explicit; planning must bind them to write scopes. | Plan write-safety gates. | repo + user |
| Tradeoff vetoes | 1 | Vetoes are explicit enough for planning. | Plan implementation constraints. | user ask |
| Scope boundary | 0 | User chose setup + init in this mission. | Handoff to plan. | user answer |
| Autonomy boundary | 1 | Codex can choose implementation details; user-only decisions are locked. | Plan decision boundaries. | clarify synthesis |
| Baseline facts | 1 | Repo facts are sufficient for planning. | Cite evidence in blueprint. | repo |
| Rollout or migration constraints | 1 | Local-machine compatibility is locked; broader release polish is out of scope. | Plan local validation. | user ask |

## Candidate Success Criteria

- `codex1 setup` configures the user's machine/global Codex runtime and does not write repo-local `.codex`, repo-local `AGENTS.md`, or Git state by default.
- `codex1 init` is an explicit project-scoped command that may configure the current repo/project when the user runs it there.
- Setup and init both create backups before modifying existing managed/user-owned files.
- `codex1 doctor` verifies the global setup and can explain missing, drifted, or unsafe state.

## Protected Surface Hypotheses

- Existing files under `~/.codex`, especially `config.toml`, `hooks.json`, skills, prompts, and any global guidance file.
- Existing uncommitted work in `/Users/joel/codex1`; do not revert or overwrite unrelated changes.
- Arbitrary project repos such as `/Users/joel/rotrut`; global setup must not mutate them unless the user explicitly runs project init there.

## Baseline Repo Facts

| Fact | Provenance | Evidence ref | Confidence |
| --- | --- | --- | --- |
| `/Users/joel/codex1` is a Rust workspace with CLI commands `setup`, `doctor`, `qualify-codex`, `restore`, `uninstall`, and internal commands. | repo-grounded | `crates/codex1/src/main.rs` | high |
| Current `codex1 setup` is project-scoped: it resolves a repo root, requires Codex trust, and plans writes to `.codex/config.toml`, `.codex/hooks.json`, `AGENTS.md`, and `.codex/skills`. | repo-grounded | `crates/codex1/src/commands/setup.rs` | high |
| Current backup infrastructure exists in `codex1-core` and supports user/project scopes in manifests, but the active setup path currently records setup changes as project-scoped. | repo-grounded | `crates/codex1-core/src/backup.rs`; `crates/codex1/src/commands/setup.rs` | high |
| `oh-my-codex` defaults setup scope to user/global and supports project scope separately, with backup context rooted under the user or project depending on scope. | repo-grounded | `/Users/joel/oh-my-codex/src/cli/setup.ts` | high |
| User wants this compatible with the current machine first, not a cross-platform release-quality implementation in this pass. | user-stated | chat request | high |

## Open Assumptions

- Global Codex1 guidance can live under `~/.codex`.
- Backup manifests under `~/.codex1/backups` are acceptable.
- Existing project setup code can be reused as the basis for `codex1 init`.

## Highest-Value Next Question

Resolved: user chose `Setup + init`.

## Feasibility Notes

- Probe used: inspected local `/Users/joel/codex1` and `/Users/joel/oh-my-codex` command structure.
- Result: the desired split is feasible without designing from scratch; current Codex1 setup can be moved behind an explicit init boundary and a new global setup can target `~/.codex`.
- Constraint surfaced: current repo has substantial uncommitted work; implementation must be scoped and non-destructive.
