# Qualification Gates

`codex1 qualify-codex` currently evaluates these gate families:

- `codex_build_probe`: capture the exact `codex --version` output used during the run and fail the live build gate unless it matches the trusted Codex baseline `0.120.0`.
- `project_config_present`: verify the target repo has project-scoped `.codex/config.toml`.
- `project_codex_hooks_enabled`: verify that project config enables `features.codex_hooks = true`.
- `project_hooks_file_present`: verify the target repo has project-scoped `.codex/hooks.json`.
- `project_stop_hook_authority`: verify exactly one authoritative Stop-hook pipeline is visible in the hooks config, while allowing additional observational Stop hooks.
- `project_agents_scaffold_present`: verify the target repo has the Codex1-managed `AGENTS.md` scaffold block.
- `project_skill_surface_valid`: verify the target repo has a valid discoverable skill surface through `copied_skills`, `linked_skills`, or `skills_config_bridge`.
- `isolated_helper_flow`: run `setup`, `doctor`, `restore`, and `uninstall` in an isolated temp repo plus isolated `HOME` / `CODEX_HOME`, then confirm the sandbox returns to baseline on the clean helper lifecycle.
- `helper_force_normalization_flow`: seed a repo with multiple project Stop handlers, prove `setup` rejects that shape without `--force`, then prove `setup --force` converges back to one authoritative managed Codex1 Stop pipeline.
- `helper_partial_install_repair_flow`: seed a deliberately partial support surface representative of an interrupted helper install, then prove rerunning `setup` repairs it to a support-ready state.
- `helper_drift_detection_flow`: drift a managed shared file after setup, then prove `doctor` surfaces the drift honestly and that `restore` / `uninstall` fail safe instead of guessing.
- `runtime_backend_flow`: run the internal mission-runtime flow in an isolated temp repo and confirm that mission artifacts, graph-backed blueprint writeback, execution packages, writer packets, review bundles, contradiction records, resume-resolution outputs, and selection consume state are all persisted.
- `waiting_stop_hook_flow`: prove that durable mission-waiting and resolver-created selection-wait states yield through the Stop hook with the canonical request exactly once before acknowledgement.
- `native_exec_resume_flow`: prove the exact trusted Codex build can create a machine-readable `codex exec` session and resume the same thread through `codex exec resume`.
- `native_multi_agent_resume_flow`: prove the exact trusted Codex build can exercise the resume-critical native child-agent inspection path across `spawn_agent`, `list_agents`, `wait_agent`, and `close_agent`, then feed the resulting live child snapshot into Codex1 resume reconciliation without false completion. Queue-only child messaging and turn-triggering delivery are recorded observationally when the build surfaces them, but they are not the decisive pass/fail signal for this resume gate.
- `manual_internal_contract_parity`: run the same mission truth through an explicit manual backend sequence and an autopilot-style backend composition, then confirm both paths converge to the same validated durable artifact summary.
- `self_hosting_source_repo`: verify the source workspace contains the expected `codex1` source markers and managed support surfaces after setup.

These gates still do not automate every possible PRD scenario, but they now provide inspectable evidence for the support surface, helper repair/fail-safe behavior, mission-runtime backend, internal backend parity, durable waiting behavior, native `codex exec resume`, native child-agent tooling, and the authoritative Stop-hook pipeline.

The supported helper baseline enforced by setup, doctor, and qualification is:

- `model = "gpt-5.4"`
- `review_model = "gpt-5.4-mini"`
- `model_reasoning_effort = "high"`
- `[codex1_orchestration] model = "gpt-5.4"`
- `[codex1_orchestration] reasoning_effort = "high"`
- `[codex1_review] model = "gpt-5.4-mini"`
- `[codex1_review] reasoning_effort = "high"`
- `[codex1_fast_parallel] model = "gpt-5.3-codex-spark"`
- `[codex1_fast_parallel] reasoning_effort = "high"`
- `[codex1_hard_coding] model = "gpt-5.3-codex"`
- `[codex1_hard_coding] reasoning_effort = "xhigh"`
- `features.codex_hooks = true`
- `agents.max_threads = 16`
- `agents.max_depth = 1`
