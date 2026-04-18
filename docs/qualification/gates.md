# Qualification Gates

`codex1 qualify-codex` currently evaluates these gate families:

- `supported_platform`: verify the current host/build combination is inside the supported qualification envelope.
- `codex_build_probe`: capture the exact `codex --version` output used during the run and fail the live build gate unless it matches the trusted Codex baseline `0.120.0`.
- `trusted_repo`: verify the target repo is explicitly trusted by Codex.
- `effective_config_baseline`: verify the trusted effective config resolves every required Codex1 baseline key to the required value.
- `project_config_present`: verify the target repo has project-scoped `.codex/config.toml`.
- `project_codex_hooks_enabled`: verify that project config enables `features.codex_hooks = true`.
- `project_hooks_file_present`: verify the target repo has project-scoped `.codex/hooks.json`.
- `user_hooks_file_valid`: verify user-level hooks config is parseable when present.
- `cross_layer_stop_hook_authority`: verify the combined user/project Stop-hook surface still resolves to one authoritative Ralph pipeline while allowing observational hooks.
- `project_stop_hook_authority`: verify exactly one authoritative Stop-hook pipeline is visible in the hooks config, while allowing additional observational Stop hooks.
- `project_agents_scaffold_present`: verify the target repo has the Codex1-managed `AGENTS.md` scaffold block.
- `project_skill_surface_valid`: verify the target repo has a valid discoverable skill surface through `copied_skills`, `linked_skills`, or `skills_config_bridge`.
- `isolated_helper_flow`: run `setup`, `doctor`, `restore`, and `uninstall` in an isolated temp repo plus isolated `HOME` / `CODEX_HOME`, then confirm the sandbox returns to baseline on the clean helper lifecycle.
- `helper_force_normalization_flow`: seed a repo with multiple project Stop handlers, prove `setup` rejects that shape without `--force`, then prove `setup --force` converges back to one authoritative managed Codex1 Stop pipeline.
- `helper_partial_install_repair_flow`: seed a deliberately partial support surface representative of an interrupted helper install, then prove rerunning `setup` repairs it to a support-ready state.
- `helper_drift_detection_flow`: drift a managed shared file after setup, then prove `doctor` surfaces the drift honestly and that `restore` / `uninstall` fail safe instead of guessing.
- `runtime_backend_flow`: run the internal mission-runtime flow in an isolated temp repo and confirm that mission artifacts, graph-backed blueprint writeback, execution packages, writer packets, review bundles, contradiction records, resume-resolution outputs, and selection consume state are all persisted.
- `waiting_stop_hook_flow`: prove that durable mission-waiting and resolver-created selection-wait states yield through the Stop hook with the canonical request exactly once before acknowledgement.
- `control_loop_boundary`: prove the installed Stop-hook surface is safe because Ralph enforcement is lease-scoped: no-lease parent turns yield, subagent turns yield, active parent loop leases block on owed review gates, and paused leases yield again.
- `native_stop_hook_live_flow`: prove the trusted build dispatches the repo-local Ralph Stop hook through a real native Codex run when live qualification is enabled.
- `native_exec_resume_flow`: prove the exact trusted Codex build can create a machine-readable `codex exec` session and resume the same thread through `codex exec resume`.
- `native_multi_agent_resume_flow`: prove the exact trusted Codex build can exercise the resume-critical native child-agent inspection path across `spawn_agent`, `list_agents`, `wait_agent`, and `close_agent`, then feed the resulting live child snapshot into Codex1 resume reconciliation without false completion. Queue-only child messaging and turn-triggering delivery are recorded observationally when the build surfaces them, but they are not the decisive pass/fail signal for this resume gate.
- `review_loop_decision_contract`: prove the parent `$review-loop` branch decisions for clean continuation, non-clean repair before the cap, and six consecutive non-clean loops routing to replan.
- `reviewer_capability_boundary`: prove contaminated child-review writeback is rejected, frozen review evidence snapshots validate, and clean parent-owned snapshot-backed review writeback still passes.
- `delegated_review_authority`: prove public docs forbid parent self-review and durable review writeback rejects missing reviewer-agent output evidence or missing review truth snapshots.
- `manual_internal_contract_parity`: run the same mission truth through an explicit manual backend sequence and an autopilot-style backend composition, then confirm both paths converge to the same validated durable artifact summary, gate outcomes, and verdict family.
- `self_hosting_source_repo`: verify the source workspace contains the expected `codex1` source markers and managed support surfaces after setup.

These gates still do not automate every possible PRD scenario, but they now provide inspectable evidence for the support surface, helper repair/fail-safe behavior, mission-runtime backend, internal backend parity, durable waiting behavior, native `codex exec resume`, native child-agent tooling, and the authoritative Stop-hook pipeline. Together they form the autonomy-governance proof surface for execute and autopilot.

That proof surface is broader than any single gate. In particular,
`manual_internal_contract_parity` is supporting evidence for public
execute/autopilot honesty, not the sole proof of the public skill-level
mission-close routing contract.

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
