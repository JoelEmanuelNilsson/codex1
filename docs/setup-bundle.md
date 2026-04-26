# Setup Bundle

Codex1 setup treats the product as one bundle:

- `codex1` CLI commands
- the Ralph Stop-hook adapter
- Codex1 skills
- Codex1 guidance
- mission artifact conventions

Global setup means the bundle is available on the machine. It does not mean the bundle is active in every repository. The default setup path is global availability plus activation for only the current repo through an allowlist policy.

Activation is mechanical and reversible. Setup edits known Codex integration points, writes Codex1-owned policy, materializes owned repo-scoped skill and guidance files, creates backups before mutation, and reports effective activation. It does not decide mission truth, artifact correctness, review state, close readiness, or PRD satisfaction.

Repo disable, uninstall, and migration must not delete mission artifacts. They may remove only Codex1-managed hook entries and Codex1-managed repo bundle files.
