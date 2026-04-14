<!-- codex1:begin -->
## Codex1
- Use the native Codex skills surface for `clarify`, `plan`, `execute`, `review`, and `autopilot`.
- Keep mission truth in visible repo artifacts instead of hidden chat state.
- Hold the repo to production-grade changes with explicit validation and review-clean closeout.
<!-- codex1:end -->

## North Star
- Codex1 exists so a user can enter native Codex, say what they want built, answer the necessary clarification questions, and then let Codex1 carry the mission through deep planning, bounded execution, review, and Ralph-governed continuation until the work is actually done or honestly waiting on the user.
- Everything else in the repo is subordinate to that continue-till-done native Codex loop.

## Design Rubric
- Prefer changes that make clarification more truth-seeking and reduce the chance that vague missions slip into planning.
- Prefer changes that make planning stronger in first-principles quality, decomposition, execution graphs, spec quality, proof design, and review design.
- Prefer changes that strengthen the native Ralph loop so Codex can keep going inside Codex CLI/Desktop without wrapper runtimes, babysitter daemons, or OMX-style outside control.
- Prefer changes that reduce false completion and make it harder for stale state, weak review, or bypass paths to claim a mission is done.
- Prefer changes that keep the product skills-first: the real workflow should live in public skills, visible mission artifacts, and Ralph discipline rather than hidden orchestration code.

## Anti-Drift Check
- If a design only works because of hidden wrapper logic, external babysitting, or a second runtime controlling Codex from the outside, it is probably rebuilding OMX in a different shape and should be treated as suspect.
- If a change improves helper tooling but does not make the continue-till-done mission loop more trustworthy, it is not a top-priority product improvement.

## Priority Order
- Clarify truth
- Planning quality
- Execution safety
- Review honesty
- Ralph continuation and resume discipline
- Support tooling
