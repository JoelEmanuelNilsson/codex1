# Round 14 Meta-Review

Baseline under review: `d88ecda3f1098a4cc8eb4bff2a0e9368da762d49`

Round 14 was not clean. Sixteen reviewer agents reported raw findings in `docs/audits/round-14/findings.md`. Eight finding-review shards reviewed those candidates. Nine findings are accepted this round: one P1 and eight P2s. The remaining candidates were confirmed as continuations of already-accepted round-8/round-10/round-11/round-12/round-13 families rather than new standalone defect lines.

Clean-round counter: reset to 0.

## Accepted Findings

| ID | Verdict | Severity | Final title | Notes |
| --- | --- | --- | --- | --- |
| F02 | Accepted | P2 | `task next` docs still advertise `REPLAN_REQUIRED` as an error even though the runtime returns a success envelope with `next.kind = "replan"` | Public contract drift on a live orchestration surface. |
| F03 | Accepted | P2 | Task lifecycle docs still publish PascalCase statuses that the runtime no longer emits | Public JSON examples are now materially wrong. |
| F04 | Accepted | P2 | Bare mission discovery still counts symlinked non-missions as real candidates | A single valid mission can be spuriously treated as ambiguous. |
| F06 | Accepted | P2 | `outcome ratify` rejects valid indented YAML frontmatter even though `outcome check` accepts it as ratifiable | Check/ratify disagree on a legal frontmatter shape. |
| F07 | Accepted | P2 | Forbidden workflow-policy fields can still be ratified into `OUTCOME.md` | Clarify/runtime still bless workflow policy as mission destination truth. |
| F12 | Accepted | P1 | Locked-plan execution/readiness surfaces still ignore `state.plan.hash`, so post-lock `PLAN.yaml` edits can change live work without replan/relock | The locked DAG can still be bypassed by out-of-band plan edits. |
| F16 | Accepted | P2 | Ralph hook still fails open on explicit selector errors (`CODEX1_MISSION`, `CODEX1_REPO_ROOT`) | Explicit operator misconfiguration is still hidden as allow-stop. |
| F17 | Accepted | P2 | `$autopilot`’s published flow still skips the required `close check` gate before `close complete` | Public orchestration guidance is still self-contradictory on the terminal-close step. |
| F18 | Accepted | P2 | `$review-loop`’s mission-close workflow still requires undefined `CLOSEOUT-preview` / proof-index artifacts | Public mission-close reviewer workflow still asks for artifacts the product never exposes. |

## Dropped, Merged, Or Non-Standalone Findings

| ID | Final disposition | Reason |
| --- | --- | --- |
| F01 | Merge target | Same replan-gate family as round-11 F10 / round-13 F10. The live contradiction is still there, but not as a new round-14 standalone line. |
| F05 | Merge target | Same `CLOSEOUT.md` mission-close history truth family as round-11 F18 / round-12 F18. The symlinked `reviews/` poison path is another manifestation of that closeout-history problem. |
| F08 | Merge target | Same superseded-dirty-review blocker family as round-11 F14 / round-13 F15/F17. |
| F09 | Merge target | Same stale-after-replan audit-loss family as round-10 F14 / round-11 merged F15 / round-13 F13. |
| F10 | Merge target | Same dirty-review-overadvertised-before-repair family as round-11 F09 / round-12 F08 / round-13 F09. |
| F11 | Merge target | Same non-target review-dependency over-admission family as round-10 F16 / round-11 merged F27 / round-13 F16. |
| F13 | Merge target | Same planned-review restart boundary family as round-11 F19 / round-12 F13 / round-13 F12. |
| F14 | Merge target | Same mission-close round-identity family as round-11 F20 / round-12 F21 / round-13 F17. |
| F15 | Merge target | Same artifact publication before commit family as round-12 F14 / round-13 F08. |
| F19 | Merge target | Same custom-install / spaced-`INSTALL_DIR` family as round-8 F13/F14 / round-12 F17 / round-13 F21. |
| F20 | Merge target | Same ambiguous-Ralph family as round-8 F11, now as lingering docs/runtime drift rather than the old fail-open runtime bug. Downgraded from candidate P1 to P2 drift. |
| F21 | Merge target | Same stale “Phase B / NOT_IMPLEMENTED” docs family as round-6 P2-8. |

## Main-Thread Agreement

The main thread accepts the finding-review verdicts.

Round 14 is not clean for four main reasons:

1. Locked-plan truth is still not fully authoritative at runtime:
   - execution/readiness surfaces still ignore `state.plan.hash`,
   - post-lock `PLAN.yaml` edits can still change live work before any recorded replan/relock.
2. Outcome truth still has two contract holes:
   - ratify still disagrees with check on valid indented frontmatter,
   - forbidden workflow-policy fields can still be ratified into `OUTCOME.md`.
3. Mission/operator path handling still has two sharp edges:
   - bare mission discovery still counts symlinked fake missions,
   - Ralph still fails open on explicit selector errors.
4. Several operator- and skill-facing docs remain materially wrong enough to mislead orchestration:
   - `task next` and task-status docs drift,
   - `$autopilot` skips the published `close check` gate,
   - `$review-loop` mission-close guidance still requires nonexistent preview artifacts.

## Repair Priorities

1. Restore locked-plan authority:
   - F12.
2. Tighten outcome contract enforcement:
   - F06, F07.
3. Fix mission/operator path handling:
   - F04, F16.
4. Correct public docs/skill guidance tied to live behavior:
   - F02, F03, F17, F18.
5. While touching adjacent codepaths, fold in the already-open merge families they overlap with:
   - round-11 F09, F10, F14, F19, F20,
   - round-12 F14, F18,
   - round-13 F08, F09, F10, F12, F13, F15, F16, F17, F21.
