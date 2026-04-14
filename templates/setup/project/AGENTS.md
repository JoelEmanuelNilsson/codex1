# Repo Instructions

<!-- CODEX1:BEGIN MANAGED BLOCK -->
## Workflow Stance

- Use Codex1 public workflows as the normal surface: `$clarify`, `$plan`,
  `$execute`, `$review`, and `$autopilot`.
- Treat `PLANS/` as visible mission truth and `.ralph/` as machine state.
- Keep `AGENTS.md` thin. Mission-specific planning details do not belong here.
- Replan is internal. Do not ask the user to chain it manually during normal
  flow.

## Quality Bar

- No "probably done" states.
- Work is complete only when the locked outcome, proof, review, and closeout
  contracts are all satisfied.
- Review is mandatory before mission completion.
- Prefer PR-ready output, and create the PR when the repo context allows it.

## Repo Commands

- Build: `{{BUILD_COMMAND}}`
- Test: `{{TEST_COMMAND}}`
- Lint or format: `{{LINT_COMMAND}}`
- Additional check 1: `{{ADDITIONAL_CHECK_1}}`
- Additional check 2: `{{ADDITIONAL_CHECK_2}}`

## Artifact Conventions

- Mission packages live under `PLANS/<mission-id>/`.
- `README.md` is summary only.
- `OUTCOME-LOCK.md` is canonical for destination truth.
- `PROGRAM-BLUEPRINT.md` is canonical for route truth.
- `specs/*/SPEC.md` is canonical for one bounded execution slice.
- `REVIEW-LEDGER.md` and `REPLAN-LOG.md` record readable review and replan
  history when those events exist.
<!-- CODEX1:END MANAGED BLOCK -->

## Repo-Specific Notes

- {{REPO_SPECIFIC_NOTES}}
