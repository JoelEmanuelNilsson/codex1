# Spec Review Notes

## Spec

- Mission id: `global-codex1-installer`
- Spec id: `cli_command_split`
- Current review bundle: `not-opened`
- Bundle kind: `spec_review`
- Source package id: `911b7252-9f9b-4c38-88fa-ba58562f3520`
- Governing refs: pending
- Lock fingerprint: `pending`
- Blueprint fingerprint: `pending`
- Spec revision and fingerprint: `pending` / `pending`
- Review lenses: command UX clarity; behavior preservation; protected-surface safety; test adequacy; qualification helper smoke compatibility
- Proof rows under review: setup global boundary; init project behavior; qualification helper smoke compatibility; relevant cargo subset
- Receipts: `RECEIPTS/2026-04-17-cli-command-split.md`
- Changed files or diff: `crates/codex1/src/main.rs`, `crates/codex1/src/commands/mod.rs`, `crates/codex1/src/commands/setup.rs`, `crates/codex1/src/commands/init.rs`, `crates/codex1/src/commands/qualify.rs`, `crates/codex1/tests/qualification_cli.rs`
- Touched interface contracts: public CLI `setup`/`init`; qualification helper smoke command invocations
- Bundle freshness status: execution receipts current; review not yet run

## Review Events

| Review id | Kind | Reviewer | Governing refs | Verdict |
| --- | --- | --- | --- | --- |
| No review events recorded yet. | spec_review | No review events recorded yet. | No review events recorded yet. | No review events recorded yet. |

## Findings

| Class | Summary | Blocking | Evidence refs | Disposition |
| --- | --- | --- | --- | --- |
| No review events recorded yet. | No review events recorded yet. | No review events recorded yet. | No review events recorded yet. | No review events recorded yet. |

## Remaining Blockers

- Spec review has not been run yet.

## Review Event `f5f1d621-c8a0-4eb9-bab6-0ebd1bb01659`

### Spec

- Mission id: `global-codex1-installer`
- Spec id: `cli_command_split`
- Bundle id: `dbef8e3c-3ba7-4e85-9d2e-55b99b01d6c9`
- Source package id: `911b7252-9f9b-4c38-88fa-ba58562f3520`

### Review Events

| Review id | Kind | Reviewer | Governing refs | Bundle id | Source package id | Lock fp | Blueprint fp | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| f5f1d621-c8a0-4eb9-bab6-0ebd1bb01659 | spec_review | parent-review-loop | package:911b7252-9f9b-4c38-88fa-ba58562f3520, bundle:dbef8e3c-3ba7-4e85-9d2e-55b99b01d6c9 | dbef8e3c-3ba7-4e85-9d2e-55b99b01d6c9 | 911b7252-9f9b-4c38-88fa-ba58562f3520 | sha256:7314d24eea35d8d5439ddb2d0e3d87ac3c9af728e1eb2d2c1336c2eb5ac9cf39 | sha256:cf607369dc06a4f92454ca033aee0c3874e3e62defde3c53afca493883de6e06 | repair_required |

### Findings

| Class | Summary | Blocking | Evidence refs | Disposition |
| --- | --- | --- | --- | --- |
| B-Spec | Doctor remediation still points project repair at setup | yes | reviewer-output:dbef8e3c-3ba7-4e85-9d2e-55b99b01d6c9:3b2781e1-da5b-4ca1-ae16-6edaabb8ad45, reviewer-output:dbef8e3c-3ba7-4e85-9d2e-55b99b01d6c9:25061cc1-9264-493b-b867-aeff2ee32d65 | targeted_repair_required |
| B-Spec | Setup accepts repo-root but ignores it | yes | reviewer-output:dbef8e3c-3ba7-4e85-9d2e-55b99b01d6c9:25061cc1-9264-493b-b867-aeff2ee32d65 | targeted_repair_required |

## Review Event `c6d22d7a-a744-4295-8fcf-2e1b2993917c`

### Spec

- Mission id: `global-codex1-installer`
- Spec id: `cli_command_split`
- Bundle id: `4c7761c4-7b50-44a0-8506-2f9bdbcab4d5`
- Source package id: `7d2bc979-377d-46d0-9738-4847ddaed935`

### Review Events

| Review id | Kind | Reviewer | Governing refs | Bundle id | Source package id | Lock fp | Blueprint fp | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| c6d22d7a-a744-4295-8fcf-2e1b2993917c | spec_review | parent-review-loop | package:7d2bc979-377d-46d0-9738-4847ddaed935, bundle:4c7761c4-7b50-44a0-8506-2f9bdbcab4d5 | 4c7761c4-7b50-44a0-8506-2f9bdbcab4d5 | 7d2bc979-377d-46d0-9738-4847ddaed935 | sha256:7314d24eea35d8d5439ddb2d0e3d87ac3c9af728e1eb2d2c1336c2eb5ac9cf39 | sha256:cf607369dc06a4f92454ca033aee0c3874e3e62defde3c53afca493883de6e06 | clean |

### Findings

| Class | Summary | Blocking | Evidence refs | Disposition |
| --- | --- | --- | --- | --- |
| NB-Note | No findings recorded | no | none | n/a |

