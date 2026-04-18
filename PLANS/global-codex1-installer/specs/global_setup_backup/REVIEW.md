# Spec Review Notes

## Spec

- Mission id: `global-codex1-installer`
- Spec id: `global_setup_backup`
- Current review bundle: `not-opened`
- Bundle kind: `spec_review`
- Source package id: `4928be7e-3f5e-43e2-834c-41e6f8c402f4`
- Governing refs: pending
- Lock fingerprint: `pending`
- Blueprint fingerprint: `pending`
- Spec revision and fingerprint: `pending` / `pending`
- Review lenses: correctness; protected-surface safety; backup/restore integrity; test adequacy; command UX clarity
- Proof rows under review: user-scope config/hooks setup; user-scope backup manifest; idempotence; relevant cargo subset
- Receipts: `RECEIPTS/2026-04-17-global-setup-backup.md`
- Changed files or diff: `crates/codex1/src/commands/setup.rs`, `crates/codex1/tests/qualification_cli.rs`
- Touched interface contracts: global `codex1 setup` report; user-scope backup manifest entries
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

## Review Event `29c04546-5465-41fc-8852-4a7a80d0f9f9`

### Spec

- Mission id: `global-codex1-installer`
- Spec id: `global_setup_backup`
- Bundle id: `137ba22b-7522-482f-a43e-478340c88864`
- Source package id: `4928be7e-3f5e-43e2-834c-41e6f8c402f4`

### Review Events

| Review id | Kind | Reviewer | Governing refs | Bundle id | Source package id | Lock fp | Blueprint fp | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 29c04546-5465-41fc-8852-4a7a80d0f9f9 | spec_review | parent-review-loop | package:4928be7e-3f5e-43e2-834c-41e6f8c402f4, bundle:137ba22b-7522-482f-a43e-478340c88864 | 137ba22b-7522-482f-a43e-478340c88864 | 4928be7e-3f5e-43e2-834c-41e6f8c402f4 | sha256:7314d24eea35d8d5439ddb2d0e3d87ac3c9af728e1eb2d2c1336c2eb5ac9cf39 | sha256:ad18469c285fdbac0fdfd6a4d22ac08fba083395d6c78735633e881030790b37 | repair_required |

### Findings

| Class | Summary | Blocking | Evidence refs | Disposition |
| --- | --- | --- | --- | --- |
| B-Spec | Global setup backups cannot be restored by existing CLI | yes | reviewer-output:137ba22b-7522-482f-a43e-478340c88864:4c1f48c4-ec6b-4899-bfb0-981030fb54c5 | targeted_repair_required |
| B-Proof | Global setup rewrites semantically desired config/hooks | yes | reviewer-output:137ba22b-7522-482f-a43e-478340c88864:b6520f1d-d379-4bbc-98b2-299a8c5bbb77 | targeted_repair_required |

## Review Event `28aa907b-c6a6-487c-9348-67501e7cd68d`

### Spec

- Mission id: `global-codex1-installer`
- Spec id: `global_setup_backup`
- Bundle id: `ef750bd1-176e-4de9-a268-4db429d0ed11`
- Source package id: `7771db17-e3db-422b-88d3-ff343f8c68a9`

### Review Events

| Review id | Kind | Reviewer | Governing refs | Bundle id | Source package id | Lock fp | Blueprint fp | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 28aa907b-c6a6-487c-9348-67501e7cd68d | spec_review | parent-review-loop | package:7771db17-e3db-422b-88d3-ff343f8c68a9, bundle:ef750bd1-176e-4de9-a268-4db429d0ed11 | ef750bd1-176e-4de9-a268-4db429d0ed11 | 7771db17-e3db-422b-88d3-ff343f8c68a9 | sha256:7314d24eea35d8d5439ddb2d0e3d87ac3c9af728e1eb2d2c1336c2eb5ac9cf39 | sha256:ad18469c285fdbac0fdfd6a4d22ac08fba083395d6c78735633e881030790b37 | clean |

### Findings

| Class | Summary | Blocking | Evidence refs | Disposition |
| --- | --- | --- | --- | --- |
| NB-Note | No findings recorded | no | none | n/a |

## Review Event `7a5a26b9-38fa-4220-a1f6-0cfcf60260b7`

### Spec

- Mission id: `global-codex1-installer`
- Spec id: `global_setup_backup`
- Bundle id: `26194473-3804-46cd-b8af-af07f48d85ba`
- Source package id: `a8953e94-436b-4408-8d13-6611da826969`

### Review Events

| Review id | Kind | Reviewer | Governing refs | Bundle id | Source package id | Lock fp | Blueprint fp | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 7a5a26b9-38fa-4220-a1f6-0cfcf60260b7 | spec_review | parent-review-loop | lock:1, blueprint:6, bundle:26194473-3804-46cd-b8af-af07f48d85ba, spec:global_setup_backup:4 | 26194473-3804-46cd-b8af-af07f48d85ba | a8953e94-436b-4408-8d13-6611da826969 | sha256:7314d24eea35d8d5439ddb2d0e3d87ac3c9af728e1eb2d2c1336c2eb5ac9cf39 | sha256:6250fe0d1ab5eda605be0adda7974655b796c3b253cd9c190d6c2b78b2b0bda7 | repair_required |

### Findings

| Class | Summary | Blocking | Evidence refs | Disposition |
| --- | --- | --- | --- | --- |
| B-Proof | Global setup proof overclaims uninstall and human-output coverage | yes | reviewer-output:26194473-3804-46cd-b8af-af07f48d85ba:57ab1927-59e2-4f8c-bc47-76bd42f707a5, PLANS/global-codex1-installer/specs/global_setup_backup/SPEC.md:98-105, PLANS/global-codex1-installer/specs/global_setup_backup/RECEIPTS/2026-04-17-global-setup-replan-proof.md:14-20, crates/codex1/tests/qualification_cli.rs:245-352, crates/codex1/tests/qualification_cli.rs:477-528 | targeted_repair_required |

## Review Event `1d30ed88-39f6-42bd-8b97-5d17f77442bd`

### Spec

- Mission id: `global-codex1-installer`
- Spec id: `global_setup_backup`
- Bundle id: `400ef148-6488-44d1-8508-ed11df12f6d5`
- Source package id: `3b551258-80ca-4ac0-85eb-5039513edc76`

### Review Events

| Review id | Kind | Reviewer | Governing refs | Bundle id | Source package id | Lock fp | Blueprint fp | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1d30ed88-39f6-42bd-8b97-5d17f77442bd | spec_review | parent-review-loop | package:3b551258-80ca-4ac0-85eb-5039513edc76, bundle:400ef148-6488-44d1-8508-ed11df12f6d5, spec:global_setup_backup:4 | 400ef148-6488-44d1-8508-ed11df12f6d5 | 3b551258-80ca-4ac0-85eb-5039513edc76 | sha256:7314d24eea35d8d5439ddb2d0e3d87ac3c9af728e1eb2d2c1336c2eb5ac9cf39 | sha256:6250fe0d1ab5eda605be0adda7974655b796c3b253cd9c190d6c2b78b2b0bda7 | blocked |

### Findings

| Class | Summary | Blocking | Evidence refs | Disposition |
| --- | --- | --- | --- | --- |
| B-Proof | Human changed-path proof is not isolated from preflight output | yes | reviewer-output:400ef148-6488-44d1-8508-ed11df12f6d5:8fe773a2-bf5d-4a55-b254-a06d9bdba389 | targeted_repair_required |

## Review Event `812d7030-daf4-4733-bfc5-cf8126e45d6b`

### Spec

- Mission id: `global-codex1-installer`
- Spec id: `global_setup_backup`
- Bundle id: `4882c015-9794-4465-8d8f-d7b0b869ef3b`
- Source package id: `41ee7737-6785-4a4b-a447-1fd3c1138e65`

### Review Events

| Review id | Kind | Reviewer | Governing refs | Bundle id | Source package id | Lock fp | Blueprint fp | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 812d7030-daf4-4733-bfc5-cf8126e45d6b | spec_review | parent-review-loop | package:41ee7737-6785-4a4b-a447-1fd3c1138e65, bundle:4882c015-9794-4465-8d8f-d7b0b869ef3b, spec:global_setup_backup:4 | 4882c015-9794-4465-8d8f-d7b0b869ef3b | 41ee7737-6785-4a4b-a447-1fd3c1138e65 | sha256:7314d24eea35d8d5439ddb2d0e3d87ac3c9af728e1eb2d2c1336c2eb5ac9cf39 | sha256:6250fe0d1ab5eda605be0adda7974655b796c3b253cd9c190d6c2b78b2b0bda7 | clean |

### Findings

| Class | Summary | Blocking | Evidence refs | Disposition |
| --- | --- | --- | --- | --- |
| NB-Note | No findings recorded | no | none | n/a |

