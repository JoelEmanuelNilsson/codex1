# Setup Bundle

Codex1 setup materializes repo-scoped artifact workflow guidance:

- a small Codex1 overview skill
- a Clarify workflow skill
- a Create PRD workflow skill
- a Plan workflow skill
- a managed repo guidance block
- a managed setup marker
- backups for touched setup files

Setup is local to the target repository. It does not install global hooks, project hooks, custom continuation adapters, continuation policy, or native goal integrations.

Activation is mechanical and reversible. Setup writes only Codex1-managed repo files, creates backups before mutation, supports dry-run plans, and reports whether the managed repo guidance and managed skills are current. It does not decide mission truth, artifact correctness, native goal state, review state, close readiness, or PRD satisfaction.

Repo disable and uninstall must not delete mission artifacts, user skills, user-authored guidance, native goal state, or legacy `.codex1/LOOP.json` files. They may remove only Codex1-managed setup files and managed guidance blocks.
