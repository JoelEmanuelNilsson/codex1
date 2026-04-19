<!-- codex1:begin -->
## Codex1 V2

### Workflow Stance
- Use the V2 skills surface — `clarify`, `plan`, `execute`, `review-loop`,
  `close`, `autopilot` — to drive every mission.
- Keep mission truth in visible repo artifacts (`OUTCOME-LOCK.md`,
  `PROGRAM-BLUEPRINT.md`, `STATE.json`, `events.jsonl`,
  `reviews/B*.json`) instead of hidden chat state.
- Ralph consumes exactly one command: `codex1 status --mission <id> --json`.

### Quality Bar
- Work is complete only when the locked outcome, proof, review, and
  mission-close contracts are all satisfied.
- Review is mandatory before mission completion; parent self-review is
  refused by the CLI.
- Hold the repo to production-grade changes with explicit validation and
  review-clean closeout.

### Repo Commands
- Build: `cargo build -p codex1`
- Test: `cargo test -p codex1`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt --all --check`

### Binary resolution
- Skills and scripts never invoke bare `codex1`. Resolve the V2 binary
  explicitly:
  ```bash
  CODEX1="$(/Users/joel/codex1/scripts/resolve-codex1-bin)"
  ```
  then use `"$CODEX1" <subcommand>`. The resolver refuses anything that
  does not expose V2's `mission-close` and `parent-loop` subcommands,
  which prevents falling through to a pre-existing V1 `codex1` on PATH.

### Artifact Conventions
- Mission packages live under `<repo-root>/PLANS/<mission-id>/`.
- `OUTCOME-LOCK.md` is canonical for destination truth.
- `PROGRAM-BLUEPRINT.md` is canonical for route truth (DAG between
  `<!-- codex1:plan-dag:start -->` / `:end -->` markers).
- `specs/T<N>/SPEC.md` is canonical for one bounded execution slice.
- `specs/T<N>/PROOF.md` is the receipt captured by `task finish`.
- `STATE.json` is authoritative operational truth; `events.jsonl` is
  audit-only.
<!-- codex1:end -->

## North Star
- Codex1 V2 exists so a user can say what they want built, answer the
  necessary clarification questions, and let the V2 skills carry the
  mission through planning, bounded execution, review, and Ralph-governed
  continuation until the work is actually done or honestly waiting on
  the user.

## Design Rubric
- Prefer changes that make clarification more truth-seeking and reduce
  the chance that vague missions slip into planning.
- Prefer changes that make planning stronger in first-principles quality,
  decomposition, execution graphs, spec quality, proof design, and
  review design.
- Prefer changes that strengthen the Ralph stop-guard so Codex and
  Claude Code can keep going without wrapper runtimes, babysitter
  daemons, or outside-the-runner control.
- Prefer changes that reduce false completion and make it harder for
  stale state, weak review, or bypass paths to claim a mission is done.
- Prefer changes that keep the product skills-first: the real workflow
  should live in public skills, visible mission artifacts, and Ralph
  discipline rather than hidden orchestration code.

## Anti-Drift Check
- If a design only works because of hidden wrapper logic, external
  babysitting, or a second runtime controlling Codex from the outside,
  it is probably rebuilding the V1 orchestration layer in a different
  shape and should be treated as suspect.
- If a change improves helper tooling but does not make the
  continue-till-done mission loop more trustworthy, it is not a
  top-priority product improvement.

## Priority Order
- Clarify truth
- Planning quality
- Execution safety
- Review honesty
- Ralph continuation and resume discipline
- Support tooling
