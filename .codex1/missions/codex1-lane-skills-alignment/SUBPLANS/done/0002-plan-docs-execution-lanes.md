## Slice Type

AFK. The lane list and workflow model are fully decided in the PRD.

## Execution Lane

standard

## Current Behavior

Codex1 docs describe the core clarify, PRD, plan, and native `/goal` flow. Ready subplan guidance does not require an execution lane, and docs do not yet explain the new lane skills or mission-scoped proof/QA model.

## Desired Behavior

Codex1 docs and templates explain that `$plan` assigns an execution lane to every ready subplan, while native `/goal` executes the mission. The docs keep the flow simple and make specialist lanes optional.

## Key Interfaces

- `$plan`
- `SUBPLAN-BRIEF.md`
- `GOAL-BRIEF-FORMAT.md`
- Codex1 workflow docs
- Plan template sections

## Scope

- Add required `Execution Lane` guidance to subplan format docs.
- List allowed lanes: `tdd`, `diagnose`, `improve-codebase-architecture`, `prototype`, `proof-qa`, `standard`.
- Update workflow/setup docs to explain core skills versus lane skills.
- Explain mission-scoped proof/QA without reviving broad dogfood.
- Keep native `/goal` boundary explicit.

## Out Of Scope

- Creating dependency graph machinery.
- Making every mission use every lane.
- Adding product ceremony beyond the existing Codex1 flow.
- Changing the meaning of native `/goal`.

## Dependencies

- `SPECS/0001-codex1-lane-setup-contract.md`

## Blocked By

None.

## Acceptance Criteria

- [ ] Every ready subplan format path requires `Execution Lane`.
- [ ] Docs explain when `standard` is valid.
- [ ] Docs explain that `$plan` assigns lanes and `/goal` executes.
- [ ] Docs explain proof/QA as mission-scoped acceptance proof.
- [ ] Docs preserve the short user-facing flow.

## Expected Proof

- Text assertions in tests or direct proof notes showing the lane list and native goal boundary appear in installed docs.
- `cargo test`.

## Exit Criteria

Future plans can produce lane-tagged subplans without making the workflow feel heavy.
