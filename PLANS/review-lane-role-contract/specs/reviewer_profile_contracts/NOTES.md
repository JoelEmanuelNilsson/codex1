# Spec Notes

- Mission id: `review-lane-role-contract`
- Spec id: `reviewer_profile_contracts`

Use this file for bounded local notes, spike observations, or drafting scratch
that supports the spec but does not override it.

## Active Notes

- Added a visible profile matrix to `$review-loop` for `local_spec_intent`,
  `integration_intent`, `mission_close`, and `code_bug_correctness`.
- Documented locked model routing: `gpt-5.4` for spec/intent, integration,
  PRD, and mission-close judgment; `gpt-5.3-codex` for code bug/correctness
  review; no `gpt-5.4-mini` as a default blocking-review model.
- Added child reviewer output schema and severity semantics: `NONE` or JSON
  findings; P0/P1/P2 block clean review; P3 is non-blocking by default.
- Added loop-state rules: count consecutive non-clean loops, targeted repair
  does not reset the count unless the target/contract materially changes, and
  six non-clean loops route to `internal-replan`.
- Reflected findings-only review children and model routing in
  `internal-orchestration`, `MULTI-AGENT-V2-GUIDE.md`, and
  `docs/runtime-backend.md`.

## Caution

If a note changes the actual contract, move that change into `SPEC.md` or the
appropriate higher-layer artifact instead of letting this file become hidden
truth.
