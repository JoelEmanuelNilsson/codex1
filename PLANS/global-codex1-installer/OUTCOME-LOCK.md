---
artifact: outcome-lock
mission_id: global-codex1-installer
root_mission_id: global-codex1-installer
parent_mission_id: null
version: 1
lock_revision: 1
status: locked
lock_posture: unconstrained
slug: global-codex1-installer
---

# Outcome Lock

## Objective

Make Codex1 behave like a general installer/runtime setup for Codex on this machine, inspired by oh-my-codex: global machine setup first, backups before mutation, doctor verification, and a clean split where `codex1 setup` configures the user's machine/Codex runtime and `codex1 init` explicitly configures the current project.

## Done-When Criteria

- `codex1 setup` has a clearly global machine-level contract and does not mutate the current project by default.
- `codex1 init` exists as an explicit project-scoped command path for current-project configuration.
- Any existing user-owned Codex configuration that setup changes is backed up before mutation.
- `codex1 doctor` can verify the resulting global setup and explain missing, drifted, or unsafe state.

## Success Measures

- A user can run one setup command on this macOS machine and get the intended Codex1 runtime files under the appropriate user-level Codex location.
- Re-running setup is idempotent or reports exactly what changed.
- Existing non-Codex1 user config/hooks are preserved unless explicitly overridden by a force flag with visible reporting.
- Validation commands prove the selected setup, init, doctor, and backup behavior.

## Protected Surfaces

- Existing `~/.codex` files and directories, especially `config.toml`, `hooks.json`, skills, prompts, and any global guidance file.
- Existing uncommitted work in `/Users/joel/codex1`.
- Arbitrary project repos; global setup must not write repo-local `.codex`, `AGENTS.md`, or Git state by default.

## Unacceptable Tradeoffs

- Do not solve this by committing scaffolding into every repo.
- Do not silently overwrite existing user Codex config or hooks.
- Do not leave placeholder commands or fake proof.
- Do not revert unrelated uncommitted Codex1 work.

## Non-Goals

- No focus on `/Users/joel/rotrut` beyond using it as evidence of the failure mode.
- No cross-platform installer polish beyond compatibility with this machine unless planning finds it essentially free.
- No remote publishing or package release in this mission unless explicitly reopened.

## Autonomy Boundary

- Codex may decide later without asking: Rust module organization, backup manifest mechanics, exact human-readable report shape, test fixture structure, and whether code is reused or wrapped internally.
- Codex must ask before deciding: whether setup may force-normalize existing authoritative user hooks without an explicit force flag; whether to publish/release the CLI.

## Locked Field Discipline

The fields above for objective, done-when criteria, protected surfaces,
unacceptable tradeoffs, non-goals, autonomy boundary, and reopen conditions are
locked fields. Change them only through an explicit reopen or superseding lock
revision, never by silent mutation.

Baseline facts and rollout or migration constraints are also revision-gated:
extend them only through an explicit lock revision when new truth materially
changes the destination contract.

## Baseline Current Facts

- Current Codex1 setup is project-scoped and writes repo-local support files.
- Oh-my-codex provides the inspiration model: user/global setup as default, project setup as a distinct scope, backups, doctor, and uninstall.
- The current machine has both `/Users/joel/codex1` and `/Users/joel/oh-my-codex` available locally.

## Rollout Or Migration Constraints

- Implementation must be compatible on this macOS machine first.
- Preserve existing uncommitted Codex1 work.
- Make changes easy to inspect and reversible.

## Remaining Low-Impact Assumptions

- Global Codex1 guidance can live under `~/.codex`.
- Backup manifests under `~/.codex1/backups` are acceptable.
- Existing project setup code can be reused as the basis for `codex1 init`.

## Feasibility Constraints

Use this section only when `lock_posture = constrained`.

- None.

## Reopen Conditions

- User decides global `setup` should also mutate projects by default.
- User decides project `init` must be deferred after planning has included it.
- Feasibility probe shows Codex does not read the chosen global guidance location.

## Provenance

### User-Stated Intent

- The setup should be general and work like an installer on the user's machine, taking big inspiration from oh-my-codex.
- `rotrut` was only a test repo and should not drive the design.
- Backups are required.
- User selected `Setup + init`: implement global `codex1 setup` and explicit project `codex1 init` in this mission.

### Repo-Grounded Facts

- Existing Codex1 setup is project-scoped and trust-gated.
- Existing Codex1 backup infrastructure can record managed paths and before/after hashes.

### Codex Clarifying Synthesis

- A correct implementation separates global runtime configuration from per-project source control state.
- The plan should reuse existing project-scoped behavior only behind the explicit `init` command, while making default setup global and non-repo-mutating.
