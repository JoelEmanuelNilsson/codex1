# Codex1 Refactor Plan: Setup And Bundle Activation

## Problem Statement

Codex1 now has the right core authority boundary: it is a deterministic artifact helper for native Codex, not a semantic workflow engine. The current code already has mission layout, built-in artifact interviews, inventory-only inspection, mission-local events, explicit loop state, Ralph Stop-hook behavior, and doctor checks.

The next product gap is setup.

Users should not have to manually edit Codex configuration, copy skills, wire hooks, remember where Codex1 is active, or guess why Ralph did or did not run. They need a friendly setup surface that makes the Codex1 way of working available on the machine, then activates it only where they choose.

The developer wants Codex1 to behave as a bundle:

- the `codex1` CLI
- the Ralph Stop-hook adapter
- Codex1 skills
- Codex1 guidance or instructions
- mission artifact conventions

The tricky part is avoiding two bad outcomes:

- Global install accidentally changes every Codex session.
- Setup grows into a second workflow engine or config bureaucracy.

The setup refactor should therefore make activation simple, explicit, reversible, and mechanical. It should edit config safely, back up what it touches, explain effective activation, and make disabled repos fail open. It must not decide mission truth, artifact correctness, review state, close state, or execution readiness.

## Solution

Add a setup subsystem that owns Codex1 bundle installation and activation.

The happy path is:

```sh
codex1 setup install
```

When run from a repo, this means:

1. Install Codex1 capability at the user/global level.
2. Install or repair the managed global Ralph Stop hook in official Codex config.
3. Create or update Codex1 global activation policy.
4. Default that policy to allowlist mode.
5. Enable only the current repo.
6. Materialize the Codex1 skill/guidance bundle only for the enabled repo.
7. Back up every config file before mutation.
8. Return stable human and JSON output explaining what changed.

Global install must never mean active everywhere unless the user explicitly asks for that mode.

Add these setup commands:

- `setup install`
- `setup enable`
- `setup disable`
- `setup uninstall`
- `setup migrate`
- `setup status`
- `setup doctor`
- `setup backups list`
- `setup backups restore`

The implementation should be split into deep modules:

- setup command parsing and dispatch
- activation policy resolution
- official Codex config editing
- Codex1 global config editing
- managed hook block rendering
- repo bundle materialization
- backup creation and restoration
- setup status projection
- setup doctor diagnostics

Official Codex facts that constrain the design:

- User/global Codex config lives under the Codex home config location.
- Project Codex config lives under a repo `.codex` config location and may be skipped when the project is untrusted.
- Inline hook config uses the official `hooks.Stop` shape with command handlers.
- Stop-hook input includes `cwd` and `stop_hook_active`.
- Repo-scoped skills can be discovered from repo skill roots, including repo `.agents/skills`.
- User-level skill config supports path/name enabled flags, but project config is not a reliable per-repo toggle for globally installed user skills.

Because of that last point, the first implementation should not rely on a globally installed skill being dynamically enabled per repo. Instead, global setup should install Codex1's source bundle globally, while `setup enable` materializes repo-scoped Codex1 skills/guidance into the target repo. `setup disable` should remove or deactivate only Codex1-managed repo bundle files. This keeps skills from leaking into unrelated repos while preserving a simple global hook plus allowlist policy for Ralph.

## Commits

1. **Document the setup command contract.**
   Add setup commands, JSON envelope expectations, activation modes, backup behavior, and anti-oracle boundaries to the CLI contract docs. Keep the docs explicit that setup is mechanical activation, not mission status.

2. **Document the bundle activation model.**
   Add a short artifact/workflow note explaining that the Codex1 bundle includes CLI, Ralph, skills, guidance, and mission conventions. Clarify that global install means available globally, not active globally.

3. **Add setup command skeletons.**
   Extend the CLI parser with `setup install`, `setup enable`, `setup disable`, `setup uninstall`, `setup migrate`, `setup status`, `setup doctor`, and `setup backups`. Return stable `NOT_IMPLEMENTED`-style mechanical errors only long enough for the skeleton test, then replace them in subsequent commits.

4. **Add setup-specific error codes.**
   Introduce mechanical setup errors for invalid setup arguments, config parse failure, config write failure, backup failure, restore failure, and bundle materialization failure. Do not add semantic mission-status errors.

5. **Add setup scope types.**
   Model setup scope as global, project, or effective repo. Keep this independent from command parsing so status, doctor, install, and migrate use the same resolver.

6. **Add Codex home resolution.**
   Resolve the official Codex home and config path using the same environment conventions Codex uses where practical. Default to the user Codex config location. Add tests with an isolated fake home.

7. **Add Codex1 home resolution.**
   Resolve the Codex1 global config and backup directory. Default to a user-level Codex1 home. Add tests with an isolated fake home so setup tests never touch a real user config.

8. **Add repo resolution for setup.**
   Reuse existing repo discovery for `--repo` and current working directory. Canonicalize repo paths for global policy storage. Reject path errors mechanically.

9. **Add activation policy model.**
   Define modes `off`, `allowlist`, `denylist`, and `all`. Define repo entries with canonical path and enabled flag. Add pure unit tests for effective activation decisions.

10. **Add Codex1 global config parser.**
   Parse missing config as defaults. Parse malformed config as a setup diagnostic/error depending on command. Add tests for empty, allowlist, denylist, all, off, duplicate repo entries, and invalid mode.

11. **Add Codex1 global config writer.**
   Write stable TOML for the activation policy. Preserve only Codex1-owned config, not arbitrary official Codex config. Add tests for deterministic output.

12. **Add backup manifest model.**
   Define a backup record with id, timestamp, target kind, target path label, backup path, and reason. Do not store raw file content in the manifest. Add serialization tests.

13. **Add backup creation.**
   Before every config mutation, copy the target file if it exists and record a manifest entry. If the target does not exist, record enough metadata to restore the absence. Add tests for existing and missing target files.

14. **Add backup restoration.**
   Restore an existing-file backup or restore a previous missing-file state. Require explicit confirmation unless JSON tests pass a force flag. Add tests for both cases.

15. **Add dry-run edit planning.**
   Introduce a setup plan object that describes files to write, files to remove, backups to create, and bundle files to materialize. Dry-run commands return this plan without writing. Add tests proving no files change in dry-run mode.

16. **Add official Codex config parser/editor.**
   Use a TOML editing library that can preserve unrelated user configuration well enough for safe edits. The editor should find, insert, update, and remove only Codex1-managed hook entries.

17. **Add managed hook identity.**
   Define a stable managed hook marker or command identity so setup can detect its own hook without touching user hooks. Add tests with unrelated Stop hooks before and after the managed hook.

18. **Render the global Ralph hook command.**
   Generate a command that invokes the installed `codex1 ralph stop-hook` through an absolute or PATH-safe command strategy. The command should not embed repo-specific mission state.

19. **Add global hook install.**
   Implement the config edit that enables the official hooks feature if required and inserts or repairs the managed Stop hook. Add tests that existing unrelated config is preserved.

20. **Add global hook uninstall.**
   Remove only the managed Codex1 Stop hook and leave unrelated Stop hooks untouched. Add tests for no-op removal and multi-hook config.

21. **Add project hook install.**
   Implement project-local hook installation into the repo Codex config. Include doctor/status warnings that project-local hooks depend on official Codex project trust behavior.

22. **Add project hook uninstall.**
   Remove only the project-local managed Codex1 hook. Do not delete the repo `.codex` directory if it contains other user files. Add tests.

23. **Add Codex1 skill bundle source.**
   Add built-in Codex1 skill bodies or generated skill content as versioned resources owned by Codex1. Keep this separate from artifact templates.

24. **Add repo bundle materializer.**
   Materialize Codex1-managed repo skills/guidance into a repo-scoped skill location that official Codex can discover. Use owned filenames/directories and avoid symlinks in the first implementation.

25. **Add repo bundle deactivation.**
   Remove or disable only Codex1-managed repo bundle files. Preserve user skills and user guidance. Add tests with neighboring user skill directories.

26. **Add bundle version tracking.**
   Store a small managed marker with bundle version and generated file list. Use it to repair, update, or remove only files Codex1 owns. Add tests for stale bundle versions.

27. **Implement `setup status`.**
   Report effective activation for a repo: global config found, activation mode, repo policy result, global hook installed, project hook installed, repo bundle materialized, duplicate hook risk, backups available, and project trust caveats. Do not report mission readiness.

28. **Implement JSON status shape.**
   Add a stable machine-readable setup status object. Keep it about activation/config only. Add tests for active, inactive, disabled, uninstalled, project-local, and duplicate-hook cases.

29. **Implement `setup install` default behavior.**
   Default to global capability plus current-repo allowlist activation. Create backups, install global hook, write Codex1 policy, and materialize repo bundle. Add an end-to-end test in a fake home and fake repo.

30. **Implement `setup install --mode all`.**
   Allow explicit all-repos activation. Ensure this is never the default. Add tests proving default install is current-repo-only.

31. **Implement `setup install --scope project`.**
   Install only project-local integration and repo bundle for the target repo. Do not write the global hook unless requested. Add tests.

32. **Implement `setup enable`.**
   If global setup exists, add or enable the repo in the global policy and materialize the repo bundle. If no setup exists, run the default install behavior. Add tests.

33. **Implement `setup disable`.**
   Disable the repo in global policy when using global setup, remove/deactivate the repo bundle, and remove project hook only when project integration is the active source. Never delete mission artifacts. Add tests.

34. **Gate Ralph by activation policy.**
   Update Ralph to check setup activation before scanning loop state when setup config exists. Disabled or invalid activation policy must fail open. Existing explicit loop tests should still pass when no setup config exists.

35. **Add Ralph setup regression tests.**
   Verify an active loop blocks in an enabled repo, allows in a disabled repo, allows when policy is malformed, and allows when repo cannot be resolved.

36. **Implement `setup uninstall`.**
   Remove managed hook integration for the selected scope and optionally deactivate repo bundle. Do not delete missions or artifacts. Add tests.

37. **Implement `setup migrate --to project`.**
   Install project-local hook and repo bundle, disable this repo in global policy if requested or needed, and avoid duplicate effective hooks. Add tests.

38. **Implement `setup migrate --to global`.**
   Install global hook, enable the repo in global policy, remove project-local managed hook, and preserve repo bundle. Add tests.

39. **Implement `setup backups list`.**
   List setup backups from the manifest in human and JSON form. Add tests with multiple backup records.

40. **Implement `setup backups restore`.**
   Restore a selected backup and report what changed. Add tests for official Codex config, Codex1 global config, and repo-local config restore.

41. **Extend doctor with setup diagnostics.**
   Add checks for installed command execution from outside the checkout, global hook parseability, project hook parseability, activation policy parseability, repo bundle materialization, duplicate hooks, and backup manifest health.

42. **Add official config fixture tests.**
   Use fixtures based on official Codex hook and skill config shapes to verify setup edits parse and preserve unrelated config.

43. **Add skill activation tests.**
   Verify enabled repos receive Codex1 repo-scoped skill files and disabled repos do not. Verify user skills and unrelated repo skills are preserved.

44. **Add guidance activation tests.**
   Verify any Codex1 guidance materialized by setup is repo-scoped, managed, and removed on disable/uninstall without touching user-authored guidance.

45. **Add backup safety tests.**
   Verify every mutating setup command that writes config creates a backup first. Verify failed setup writes do not claim success without a backup.

46. **Add no-real-home tests.**
   Ensure all setup tests run against fake Codex and Codex1 homes. Add a regression test that the real user config path is never touched during tests.

47. **Update README quickstart.**
   Add a setup section showing `setup install`, `setup status`, `setup disable`, and `setup enable`. Keep the artifact workflow quickstart intact.

48. **Update skill workflow docs.**
   Explain that Codex1 skills are activated by setup and should not be assumed available outside enabled repos.

49. **Update CLI contract docs.**
   Document setup command outputs, activation modes, backup behavior, dry-run behavior, and fail-open semantics.

50. **Add anti-oracle setup regression tests.**
   Assert setup status does not emit mission next actions, task readiness, review pass/fail, proof sufficiency, close readiness, or PRD satisfaction.

51. **Run full verification and prune.**
   Run formatting, the full test suite, compile checks, and manual setup smoke tests in fake homes. Remove any setup logic that starts to look like workflow authority rather than activation/config editing.

## Decision Document

- Setup is part of the Codex1 product, not a separate installer project.
- Codex1 is treated as a bundle of CLI, Ralph hook, skills, guidance, and artifact conventions.
- Global setup means Codex1 is available on the machine.
- Global setup does not mean Codex1 is active in every repo.
- The default setup behavior is global capability plus current-repo activation.
- The default global activation mode is allowlist.
- All-repos activation must be explicit.
- Repo disable must be reversible.
- Repo disable must not delete mission artifacts.
- Setup uninstall must not delete mission artifacts.
- Setup migration must preserve mission artifacts.
- Setup edits official Codex config only for official Codex integration points.
- Codex1's own activation policy lives in Codex1-owned global config.
- Codex1 must not assume official Codex reads `.codex1` config.
- The global Ralph hook can be installed once and fail open for disabled repos.
- Ralph checks activation policy before applying loop pressure when setup policy exists.
- Invalid setup policy causes Ralph to fail open.
- Repo-scoped skill activation is preferred over globally active user skills.
- The first implementation should materialize Codex1 skills into enabled repos rather than rely on dynamic per-repo global skill toggles.
- Codex1-managed skill/guidance files must be clearly owned and safely removable.
- User-authored skills and guidance must be preserved.
- Project-local hook setup is supported for teams or repos that do not want a global hook.
- Project-local hook setup must explain official Codex project trust caveats.
- Backups are mandatory before setup mutates config.
- Dry-run is mandatory for setup mutation commands.
- Setup status explains activation/config, not mission progress.
- Setup doctor diagnoses setup health, not mission health.
- Setup commands use the same stable JSON envelope as the rest of the CLI.
- Setup events may be logged only as mechanical command metadata if the existing event policy supports them; events do not become activation truth.
- Setup remains mechanical and must not decide task readiness, review pass/fail, close readiness, proof sufficiency, or PRD satisfaction.

## Testing Decisions

- Tests should verify external command behavior and file effects rather than private implementation details.
- Setup tests must use fake Codex homes, fake Codex1 homes, and temp repos.
- No setup test may read or write the developer's real Codex config.
- Config editor tests should use realistic official Codex TOML fixtures with unrelated user settings.
- Managed hook tests should prove unrelated hooks are preserved.
- Activation policy tests should be pure and exhaustive for each mode.
- Backup tests should verify backup-before-write ordering at the observable level.
- Dry-run tests should verify no files are changed.
- Skill bundle tests should verify only Codex1-managed files are created or removed.
- Ralph integration tests should verify enabled active loops block and disabled active loops allow.
- Doctor tests should verify stale/missing executable behavior from outside the checkout.
- Migration tests should verify no duplicate effective hook remains after switching scopes.
- JSON tests should verify setup success, setup warnings, and setup errors use stable envelopes.
- Anti-oracle tests should verify setup status and doctor never emit mission workflow truth.
- Existing artifact, event, loop, Ralph, and inspect tests must continue passing.

## Out of Scope

- A graphical installer.
- A daemon.
- Automatic background setup.
- Network installation of Codex1.
- Remote plugin marketplace publishing.
- User-editable setup hook snippets.
- User-editable artifact templates.
- Automatically modifying every repo on the machine.
- Activating Codex1 globally by default.
- Loading Codex1 skills in disabled repos.
- Injecting Codex1 guidance in disabled repos.
- Deleting mission artifacts during setup changes.
- Making setup status into mission status.
- Making setup doctor into mission validation.
- Replacing official Codex configuration with Codex1-only configuration.
- Detecting whether a human or Codex initiated setup.
- Creating GitHub issues or PRs as part of setup.
- Solving official Codex trust prompts inside Codex1.

## Further Notes

The most important setup invariant is:

> Codex1 can be installed globally, but it should only behave globally when the user explicitly chooses that.

The second most important invariant is:

> Disabled means disabled: Ralph fails open, Codex1 skills are not active, and Codex1 guidance is not injected.

The setup implementation should be boring in the same way the artifact CLI is boring. It edits known config, writes backups, materializes owned files, reports what happened, and stops. It should not grow opinions about whether a mission is well planned, whether a subplan is done, or whether a PRD is satisfied.
