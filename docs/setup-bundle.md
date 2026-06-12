# Setup Bundle

Codex1 setup materializes repo-scoped artifact workflow guidance:

- a Clarify workflow skill
- a Create PRD workflow skill
- repo-local lane skills for TDD, diagnosis, and architecture improvement
- a repo-local Codex review closeout helper skill
- skill UI metadata in `agents/openai.yaml` where practical
- a managed repo guidance block
- a managed setup marker
- backups for touched setup files

Setup is local to the target repository. It does not install global skills, continuation policy, or native goal integrations.

Activation is mechanical and reversible. Setup writes only Codex1-managed repo files, creates backups before mutation, supports dry-run plans, and reports whether the managed repo guidance, managed skills, and supporting docs are current. It does not decide mission truth, artifact correctness, native goal state, review state, close readiness, or PRD satisfaction.

Repo disable and uninstall must not delete mission artifacts, user skills, user-authored guidance, or native goal state. They may remove only Codex1-managed setup files and managed guidance blocks.

## Maintenance Interface

The current Setup bundle inventory is maintained in one place: `CURRENT_BUNDLE_ENTRIES` in `src/setup/catalog.rs`. Each current file appears there once with its role and expected body. Setup status, doctor, install, uninstall, marker generation, checked-in marker drift tests, and expected-body checks derive from that interface.

`.codex1/setup-bundle.json` is an installed marker artifact, not a second source of truth. Keep it synchronized by running setup maintenance checks after changing the bundle; do not hand-maintain a separate marker file list in tests or docs.

Legacy recognition lives beside the current interface in `src/setup/catalog.rs` as compact release specs and managed-body proofs. Bias toward refusal when a legacy marker or retired file body is not known.

## Playbook

To change the body of an existing managed file:

1. Edit the managed file.
2. Run `cargo test setup::catalog -- --nocapture`, then the relevant setup integration test.
3. Run the final setup checks before publishing.

To add a managed file:

1. Add the file content.
2. Add one `CURRENT_BUNDLE_ENTRIES` entry with the right role and bump `BUNDLE_VERSION` in `src/setup/catalog.rs`.
3. Refresh `.codex1/setup-bundle.json` through setup install, then run the drift and setup checks.

To retire a managed file:

1. Remove its current entry and bump `BUNDLE_VERSION` in `src/setup/catalog.rs`.
2. Keep or add the legacy release/body proof needed to remove only known managed copies.
3. Refresh `.codex1/setup-bundle.json`, add or update a retired-file safety test, then run the drift and setup checks.

The normal edit-point budget for add/retire work is: file content, `src/setup/catalog.rs`, generated marker artifact, and docs only when category or operator guidance changes. Do not add a new manifest, generator, helper table, or test inventory unless it replaces an older authority in the same change.

## Validation

Use focused checks first:

```sh
cargo test setup::catalog -- --nocapture
cargo test --test setup
cargo test --test setup_updater
```

Close with:

```sh
cargo fmt -- --check
cargo test
cargo run --quiet -- --json setup status
cargo run --quiet -- --json setup doctor
cargo run --quiet -- --json setup install --dry-run
bash -n .agents/skills/update-codex1-setups/scripts/update-codex1-setups.sh
```

These commands prove mechanical setup behavior and drift checks. They do not prove mission readiness, proof sufficiency, native goal state, review state, or completion.
