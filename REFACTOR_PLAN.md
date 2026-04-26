# Codex1 Refactor Plan: Dumb Artifact CLI Rebuild

## Problem Statement

The current Codex1 repo has been intentionally reset around a new PRD after earlier implementations drifted into an over-smart CLI. Those earlier attempts repeatedly produced review findings because the CLI tried to own semantic workflow truth: task readiness, review cleanliness, close safety, replan priority, proof freshness, graph wave safety, and terminal completion.

The developer wants to rebuild Codex1 from scratch as a deterministic artifact workflow for native Codex. The CLI should help Codex create excellent PRDs, plans, research records, specs, subplans, ADRs, reviews, triage records, proofs, and closeouts through built-in templates and structured interviews. It should not decide whether the mission is semantically ready, complete, correct, reviewed, or safe.

The challenge is to implement enough structure to be genuinely useful without recreating the previous status oracle. The implementation must keep Codex as the semantic judge and make the CLI boring, mechanical, reliable, and easy for Codex to use.

## Solution

Build a fresh `codex1` CLI around five deep modules:

1. Mission and path safety.
2. Built-in artifact templates.
3. Interview schemas and answer validation.
4. Deterministic markdown rendering and artifact writes.
5. Explicit loop/Ralph state.

The CLI will provide commands for:

- initializing a mission directory;
- showing built-in templates;
- running artifact interviews interactively;
- running the same interviews from answers files;
- inspecting artifact inventory without semantic readiness claims;
- moving subplans between lifecycle folders;
- starting, pausing, resuming, stopping, and reading explicit loop state;
- running a Ralph Stop-hook adapter over that explicit loop state;
- running fast installation and path-safety diagnostics.

The first implementation must not include:

- `STATE.json`;
- authoritative event replay;
- task readiness computation;
- graph wave computation;
- review pass/fail computation;
- proof sufficiency computation;
- close readiness computation;
- PRD ratification;
- plan locking;
- replan state;
- any command that turns artifact contents into semantic workflow authority.

The artifact tree is the product. The CLI helps create and move files. Codex decides what those files mean.

## Commits

1. **Create the empty Rust workspace skeleton.**  
   Add a minimal package layout, build metadata, formatting configuration, and a placeholder binary that prints help. The codebase should compile and have one smoke test that invokes the binary.

2. **Add the command-line parser and global JSON envelope.**  
   Implement `--json`, `--repo-root`, and `--mission` flags. Add stable success and error envelopes. Add tests for success shape, error shape, and argument errors in JSON mode.

3. **Add canonical error types.**  
   Introduce a small error code set for argument errors, mission path errors, artifact validation errors, IO errors, template errors, interview errors, and loop errors. Keep errors mechanical and avoid semantic workflow codes like `TASK_NOT_READY` or `CLOSE_NOT_READY`.

4. **Add mission ID validation.**  
   Accept only boring mission IDs. Reject empty IDs, absolute paths, dot segments, slashes, backslashes, NUL bytes, hidden path tricks, and names that normalize outside the mission root. Add focused tests.

5. **Add repository and mission root discovery.**  
   Implement explicit repo root selection first, then current-directory discovery. Keep behavior simple and documented. Do not infer multiple active missions. Add tests for explicit repo root and cwd discovery.

6. **Add path containment helpers.**  
   Centralize safe path joins and artifact write targets. Defend against `..`, absolute paths, symlink escapes, and writing outside the mission directory. Add tests with symlinked directories and symlinked artifact targets.

7. **Implement `codex1 init`.**  
   Create a mission directory with the standard artifact folders and `.codex1/`. Do not create PRD or plan content yet. Return artifact paths in JSON mode. Add tests for idempotency and path safety.

8. **Define artifact kinds and canonical layout.**  
   Add constants and typed descriptors for PRD, research plan, research, plan, spec, subplan, ADR, review, triage, proof, closeout, loop state, and audit receipts. Add tests that descriptors render expected paths.

9. **Add the built-in template registry.**  
   Implement a registry that exposes versioned built-in templates by artifact kind. No project or user override support. Add tests that every artifact kind has exactly one v1 template.

10. **Add `codex1 template list` and `codex1 template show`.**  
   Allow Codex to inspect built-in templates. Output markdown by default and structured metadata under `--json`. Add tests for all supported artifact kinds.

11. **Create the PRD template.**  
   Add sections for title, original request, interpreted destination, success criteria, non-goals, constraints, verified context, assumptions, resolved questions, proof expectations, review expectations, and PR intent. Add golden rendering tests.

12. **Create the plan template.**  
   Add sections for mission link, strategy thesis, workstreams, phases, research posture, risk map, artifact index, review posture, and recommended next slices. Add golden rendering tests.

13. **Create the research plan template.**  
   Add sections for research questions, sources to inspect, experiments to run, expected outputs, stopping criteria, and how findings will affect the plan. Add golden rendering tests.

14. **Create the research record template.**  
   Add sections for question, sources inspected, facts found, experiments run, uncertainties, options considered, recommendation, and links to affected artifacts. Add golden rendering tests.

15. **Create the spec template.**  
   Add sections for responsibility, PRD relevance, scope, non-goals, expected behavior, interfaces/contracts, implementation notes, proof expectations, risks, and revision notes. Add golden rendering tests.

16. **Create the subplan template.**  
   Add sections for goal, linked PRD, linked plan, linked specs, owner, scope, steps, dependencies, expected proof, exit criteria, and handoff notes. Add golden rendering tests.

17. **Create the ADR template.**  
   Add sections for status, context, decision, options considered, tradeoffs, consequences, and links to PRD/plan/specs. Add golden rendering tests.

18. **Create the review template.**  
   Add sections for target artifact or code area, reviewer role, overall assessment, confidence, findings, non-blocking notes, and recommended follow-up. Add golden rendering tests.

19. **Create the triage template.**  
   Add sections for linked review, accepted findings, rejected findings, deferred findings, duplicate/stale findings, rationale, and artifact changes to make. Add golden rendering tests.

20. **Create the proof template.**  
   Add sections for linked subplan, linked spec, summary of changes, commands run, tests run, manual checks, changed areas, failures, accepted risks, and evidence links. Add golden rendering tests.

21. **Create the closeout template.**  
   Add sections for PRD satisfaction summary, completed subplans, superseded subplans, paused/deferred subplans, proofs, reviews/triage, ADRs, remaining risks, PR readiness, and final notes. Add golden rendering tests.

22. **Add section-tag rendering.**  
   Implement a small renderer that maps answers into built-in template sections. Use deterministic section identifiers. Add tests for missing section, duplicate section, and stable output.

23. **Add interview schema types.**  
   Model interview questions as data: ID, prompt, answer type, required flag, repeatability, default, and target section. Keep this module independent from terminal IO. Add unit tests for schema validation.

24. **Add answer document parsing.**  
   Support non-interactive answers files in JSON first. Validate required answers, answer types, unknown answer IDs, and repeated answers. Add tests for happy path and validation failures.

25. **Add interactive interview runner.**  
   Implement a small stdin/stdout runner that asks deterministic questions and produces the same answer structure as answers-file mode. Keep terminal behavior simple. Add tests through injected input/output streams.

26. **Add PRD interview command.**  
   Implement `codex1 interview prd`. It writes `PRD.md`. Add tests for interactive-like runner through answers file, missing required answers, safe write behavior, and JSON output.

27. **Add plan interview command.**  
   Implement `codex1 interview plan`. It writes `PLAN.md`. It may reference existing PRD but does not validate semantic sufficiency. Add tests for deterministic rendering and no semantic readiness output.

28. **Add research plan interview command.**  
   Implement `codex1 interview research-plan`. It writes `RESEARCH_PLAN.md`. Add tests.

29. **Add research record interview command.**  
   Implement `codex1 interview research`. It writes a new timestamped or numbered `RESEARCH/` file. Add tests for unique file naming.

30. **Add spec interview command.**  
   Implement `codex1 interview spec`. It writes a new `SPECS/` file. Add tests for slugging, uniqueness, and deterministic content.

31. **Add subplan interview command.**  
   Implement `codex1 interview subplan`. It writes to `SUBPLANS/ready/` by default. Add tests for lifecycle folder creation and deterministic content.

32. **Add ADR interview command.**  
   Implement `codex1 interview adr`. It writes to `ADRS/`. Add tests.

33. **Add review interview command.**  
   Implement `codex1 interview review`. It writes to `REVIEWS/`. It records opinion shape only and does not mark anything blocked. Add tests.

34. **Add triage interview command.**  
   Implement `codex1 interview triage`. It writes to `TRIAGE/`. It records main Codex adjudication but does not mutate workflow state. Add tests.

35. **Add proof interview command.**  
   Implement `codex1 interview proof`. It writes to `PROOFS/`. It does not verify that the proof proves correctness. Add tests.

36. **Add closeout interview command.**  
   Implement `codex1 interview closeout`. It writes `CLOSEOUT.md`. It does not decide PRD satisfaction. Add tests.

37. **Add artifact creation collision policy.**  
   Define how commands behave when a target artifact exists: fail by default, allow explicit overwrite flag for main artifacts, and create unique names for numbered/timestamped artifacts. Add tests.

38. **Add subplan lifecycle move command.**  
   Implement `codex1 subplan move --id <id> --to ready|active|done|paused|superseded`. This is a safe file move only. Add tests for moving, duplicate targets, unknown states, and safe paths.

39. **Allow multiple active subplans.**  
   Ensure the move command never enforces a single active subplan. Add a regression test with two active subplans.

40. **Add artifact inventory inspection.**  
   Implement `codex1 inspect`. Report which artifacts and folders exist, counts by artifact kind, and mechanical warnings such as malformed frontmatter or missing directories. Do not report readiness. Add tests.

41. **Add oracle-regression tests for inspect.**  
   Assert that inspect output does not contain fields such as `next_action`, `ready`, `complete`, `blocked`, `review_passed`, `close_ready`, `replan_required`, or `task_status`.

42. **Add audit receipt append command.**  
   Implement `codex1 receipt append` or a similarly named command for optional audit receipts. Receipts are not authority. Add tests that inspect works without receipts.

43. **Add loop state schema.**  
   Define `.codex1/LOOP.json` with version, active, paused, mode, message, pause command, and updated timestamp. Keep it tiny. Add parse and validation tests.

44. **Add loop start command.**  
   Implement `codex1 loop start --mode <mode> --message <message>`. It writes explicit loop state. Add tests.

45. **Add loop pause command.**  
   Implement `codex1 loop pause --reason <reason>`. It marks loop paused and records a reason. Add tests.

46. **Add loop resume command.**  
   Implement `codex1 loop resume`. It unpauses loop state if present. Add tests.

47. **Add loop stop command.**  
   Implement `codex1 loop stop --reason <reason>`. It marks loop inactive. Add tests.

48. **Add loop status command.**  
   Implement `codex1 loop status --json`. It reports explicit loop state only. Add tests.

49. **Add Ralph Stop-hook adapter.**  
   Implement `codex1 ralph stop-hook`. It reads official Stop-hook input, honors `stop_hook_active`, resolves mission from `cwd`, reads loop state, and blocks only when active, unpaused, and message exists. Add tests.

50. **Add Ralph fail-open tests.**  
   Cover missing loop state, corrupt loop state, inactive loop, paused loop, empty message, path errors, and `stop_hook_active == true`. All must allow stop.

51. **Add Ralph block-message tests.**  
   Verify the block output includes the continuation message and a pause/stop command. Keep wording short and useful.

52. **Add doctor command.**  
   Implement fast diagnostics: binary can run from outside checkout, templates are registered, mission path safety basics pass, loop/Ralph smoke works. Add tests with controlled PATH where practical.

53. **Add installed-command diagnostic.**  
   Verify the installed `codex1` command can emit a JSON error envelope from a temp directory without source-local environment assumptions. Add tests or an integration smoke if feasible.

54. **Add command help snapshots.**  
   Snapshot top-level help and major subcommand help enough to prevent accidental CLI drift. Keep snapshots focused.

55. **Add README quickstart.**  
   Document the concept, the artifact tree, a small mission flow, a research-heavy mission flow, loop/Ralph behavior, and the anti-oracle rule.

56. **Add artifact model documentation.**  
   Document each artifact's role, what owns it, what must not own it, and when it is created. Include examples without overbuilding a second spec forest.

57. **Add CLI contract documentation.**  
   Document commands, JSON envelope, path safety, answers-file format, and the fact that inspect is inventory-only.

58. **Add skill workflow notes.**  
   Document how future skills should use the CLI: clarify creates PRD, plan researches and creates plan/specs/subplans, execute works subplans, review-loop records reviews/triage, interrupt pauses loop, autopilot chains them.

59. **Add anti-regression design tests.**  
   Add tests or static checks that prevent `STATE.json`, `next_action`, graph wave computation, close readiness fields, or review pass/fail fields from appearing in public inspect/loop APIs.

60. **Run full verification and trim.**  
   Run formatting, linting, unit tests, integration tests, and help smoke tests. Remove accidental complexity and any command that smells like semantic workflow authority.

## Decision Document

- The rebuilt product starts from `PRD.md`.
- The previous `OUTCOME.md` concept is removed.
- The previous smart status oracle is removed.
- The first product does not include an authoritative `STATE.json`.
- The first product does not include event replay as authority.
- Optional audit receipts may exist, but they are not truth.
- Human-facing artifacts are markdown-first.
- Built-in templates are the only supported templates.
- User-editable templates are deferred.
- Every durable mission has a PRD.
- `PLAN.md` is a living strategy map.
- `PLAN.md` is mutable and current.
- `PLAN.md` is not a task tracker, proof ledger, graph scheduler, or status dashboard.
- `RESEARCH_PLAN.md` is optional and durable only for substantial research.
- `RESEARCH/` is first-class for significant investigation.
- `SPECS/` contains spec-driven development contracts for bounded responsibilities.
- Every executable slice has a subplan.
- `SUBPLANS/` uses lifecycle folders.
- Multiple active subplans are allowed.
- The CLI does not approve parallelism.
- Skills teach Codex how to reason about parallelism, ownership, risk, and delegation.
- `ADRS/` contains durable architecture decisions.
- `REVIEWS/` contains timestamped reviewer opinions.
- `TRIAGE/` contains main-Codex adjudication records.
- `PROOFS/` contains one proof per completed subplan.
- `CLOSEOUT.md` is written when Codex judges the PRD is satisfied.
- Closeout creation is not a CLI semantic gate.
- Subagents receive PRD, plan, relevant spec, relevant subplan, applicable ADRs, and explicit ownership.
- Subagents do not edit PRD or plan.
- Subagents may edit their assigned spec only when explicitly allowed.
- Subagents may propose artifact changes.
- Ralph reads only explicit loop state.
- Ralph never reads PRD, plan, specs, reviews, proofs, or closeout.
- Ralph never infers readiness or completion.
- Inspect reports inventory and mechanical warnings only.
- Inspect must not include semantic readiness fields.
- The implementation should be organized around deep modules:
  - path safety;
  - template registry;
  - interview schema and answer validation;
  - renderer;
  - artifact writer;
  - inventory inspector;
  - loop state;
  - Ralph adapter;
  - JSON envelope and errors.
- Commands should be thin wrappers over those modules.

## Testing Decisions

- Tests should emphasize public behavior: command output, generated files, path safety, and stable JSON envelopes.
- Golden tests should cover deterministic rendering for every built-in template.
- Interview tests should exercise both answers-file mode and injected interactive input.
- Path tests should be adversarial and include traversal, absolute paths, symlinks, and unsafe mission IDs.
- Inspect tests should verify inventory output and explicitly verify absence of semantic workflow fields.
- Loop tests should cover all state transitions with no dependency on other mission artifacts.
- Ralph tests should cover fail-open behavior and the one block condition.
- Subplan move tests should cover lifecycle folder movement and multiple active subplans.
- Artifact collision tests should cover existing PRD, existing plan, unique numbered artifacts, and explicit overwrite behavior where supported.
- Doctor tests should avoid relying only on `cargo run`; installed-command checks should run from a temp directory.
- End-to-end tests should cover:
  - a small mission from PRD to closeout;
  - a research-heavy mission from PRD to research plan to research to plan;
  - a spec/subplan/proof flow;
  - a review/triage flow;
  - a loop/Ralph flow.
- Regression tests should prevent reintroduction of:
  - authoritative `STATE.json`;
  - task readiness projection;
  - graph wave projection;
  - review pass/fail projection;
  - close readiness projection;
  - PRD ratification;
  - plan locking;
  - replan state.
- Tests should not require network access, GitHub, external models, or actual Codex subagents.

## Out of Scope

- GitHub issue creation.
- Writing the Codex skill files.
- User-editable templates.
- Project-specific templates.
- Template plugin systems.
- `STATE.json` as semantic authority.
- Event sourcing or replay.
- Smart status projection.
- Task lifecycle state.
- Review lifecycle state.
- Replan lifecycle state.
- Plan locking.
- PRD ratification.
- Graph wave computation.
- Close readiness computation.
- Proof sufficiency validation.
- Review correctness validation.
- A TUI.
- A daemon.
- A database.
- A wrapper runtime around Codex.
- Subagent spawning from the CLI.
- Caller identity detection.
- Fake subagent permission enforcement.
- PR creation.
- Any command that decides mission truth.

## Further Notes

The implementation should be deliberately modest. The main risk is not that the product is too small. The main risk is that the implementation agent will try to rebuild the old semantic control plane because it feels useful.

Whenever implementation reaches for a workflow verdict, stop and ask whether this belongs in Codex reasoning instead.

Good CLI questions:

- Can this path be safely written?
- Does this answers file contain required answers?
- Can this markdown artifact be rendered deterministically?
- Does this loop file explicitly request continuation?
- What artifacts exist?

Bad CLI questions:

- Is this plan correct?
- Is this subplan ready?
- Did this proof prove enough?
- Did this review pass?
- Is this mission complete?
- Should Codex replan?

The anti-oracle rule should be treated as the core engineering constraint:

```text
Codex1 preserves and structures Codex's thinking.
Codex1 does not replace Codex's thinking.
```
