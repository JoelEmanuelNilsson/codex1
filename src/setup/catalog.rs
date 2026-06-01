use serde::{Deserialize, Serialize};

use crate::error::{Codex1Error, Result};

use super::guidance;

pub(super) const BUNDLE_VERSION: u32 = 13;
pub(super) const BUNDLE_GUIDANCE: &str = "AGENTS.md";
pub(super) const BUNDLE_MARKER: &str = ".codex1/setup-bundle.json";

const OVERVIEW_SKILL: &str = ".agents/skills/codex1/SKILL.md";
const OVERVIEW_OPENAI_YAML: &str = ".agents/skills/codex1/agents/openai.yaml";
const CLARIFY_SKILL: &str = ".agents/skills/clarify/SKILL.md";
const CLARIFY_OPENAI_YAML: &str = ".agents/skills/clarify/agents/openai.yaml";
const CLARIFY_ADR_FORMAT: &str = ".agents/skills/clarify/ADR-FORMAT.md";
const CLARIFY_CONTEXT_FORMAT: &str = ".agents/skills/clarify/CONTEXT-FORMAT.md";
const CREATE_PRD_SKILL: &str = ".agents/skills/create-prd/SKILL.md";
const CREATE_PRD_OPENAI_YAML: &str = ".agents/skills/create-prd/agents/openai.yaml";
const CREATE_PRD_FORMAT: &str = ".agents/skills/create-prd/PRD-FORMAT.md";
const PLAN_SKILL: &str = ".agents/skills/plan/SKILL.md";
const PLAN_OPENAI_YAML: &str = ".agents/skills/plan/agents/openai.yaml";
const PLAN_ADR_FORMAT: &str = ".agents/skills/plan/ADR-FORMAT.md";
const PLAN_SUBPLAN_BRIEF: &str = ".agents/skills/plan/SUBPLAN-BRIEF.md";
const PLAN_GOAL_BRIEF_FORMAT: &str = ".agents/skills/plan/GOAL-BRIEF-FORMAT.md";
const TDD_SKILL: &str = ".agents/skills/tdd/SKILL.md";
const TDD_OPENAI_YAML: &str = ".agents/skills/tdd/agents/openai.yaml";
const TDD_TESTS: &str = ".agents/skills/tdd/tests.md";
const TDD_MOCKING: &str = ".agents/skills/tdd/mocking.md";
const TDD_DEEP_MODULES: &str = ".agents/skills/tdd/deep-modules.md";
const TDD_INTERFACE_DESIGN: &str = ".agents/skills/tdd/interface-design.md";
const TDD_REFACTORING: &str = ".agents/skills/tdd/refactoring.md";
const DIAGNOSE_SKILL: &str = ".agents/skills/diagnose/SKILL.md";
const DIAGNOSE_OPENAI_YAML: &str = ".agents/skills/diagnose/agents/openai.yaml";
const DIAGNOSE_HITL_LOOP_TEMPLATE: &str = ".agents/skills/diagnose/scripts/hitl-loop.template.sh";
const ARCHITECTURE_SKILL: &str = ".agents/skills/improve-codebase-architecture/SKILL.md";
const ARCHITECTURE_OPENAI_YAML: &str =
    ".agents/skills/improve-codebase-architecture/agents/openai.yaml";
const ARCHITECTURE_LANGUAGE: &str = ".agents/skills/improve-codebase-architecture/LANGUAGE.md";
const ARCHITECTURE_INTERFACE_DESIGN: &str =
    ".agents/skills/improve-codebase-architecture/INTERFACE-DESIGN.md";
const ARCHITECTURE_DEEPENING: &str = ".agents/skills/improve-codebase-architecture/DEEPENING.md";
const PROTOTYPE_SKILL: &str = ".agents/skills/prototype/SKILL.md";
const PROTOTYPE_OPENAI_YAML: &str = ".agents/skills/prototype/agents/openai.yaml";
const PROTOTYPE_LOGIC: &str = ".agents/skills/prototype/LOGIC.md";
const PROTOTYPE_UI: &str = ".agents/skills/prototype/UI.md";
const CODEX_REVIEW_SKILL: &str = ".agents/skills/codex-review/SKILL.md";
const CODEX_REVIEW_OPENAI_YAML: &str = ".agents/skills/codex-review/agents/openai.yaml";
const CODEX_REVIEW_HELPER: &str = ".agents/skills/codex-review/scripts/codex-review";
const BRUTAL_REVIEW_SKILL: &str = ".agents/skills/brutal-review/SKILL.md";
const BRUTAL_REVIEW_OPENAI_YAML: &str = ".agents/skills/brutal-review/agents/openai.yaml";
const HANDOFF_SKILL: &str = ".agents/skills/handoff/SKILL.md";
const HANDOFF_OPENAI_YAML: &str = ".agents/skills/handoff/agents/openai.yaml";
const LEGACY_PLAN_EXECUTION_PROMPT_FORMAT: &str = ".agents/skills/plan/EXECUTION-PROMPT-FORMAT.md";
const WORKFLOW_DOC: &str = "docs/agents/codex1-workflow.md";
const DOMAIN_DOC: &str = "docs/agents/codex1-domain.md";
const ARTIFACT_BRIEFS_DOC: &str = "docs/agents/codex1-artifact-briefs.md";

const MANAGED_SKILL_FILES: [&str; 11] = [
    OVERVIEW_SKILL,
    CLARIFY_SKILL,
    CREATE_PRD_SKILL,
    PLAN_SKILL,
    TDD_SKILL,
    DIAGNOSE_SKILL,
    ARCHITECTURE_SKILL,
    PROTOTYPE_SKILL,
    CODEX_REVIEW_SKILL,
    BRUTAL_REVIEW_SKILL,
    HANDOFF_SKILL,
];

const MANAGED_SUPPORTING_DOC_FILES: [&str; 32] = [
    OVERVIEW_OPENAI_YAML,
    CLARIFY_OPENAI_YAML,
    CLARIFY_ADR_FORMAT,
    CLARIFY_CONTEXT_FORMAT,
    CREATE_PRD_OPENAI_YAML,
    CREATE_PRD_FORMAT,
    PLAN_OPENAI_YAML,
    PLAN_ADR_FORMAT,
    PLAN_SUBPLAN_BRIEF,
    PLAN_GOAL_BRIEF_FORMAT,
    TDD_OPENAI_YAML,
    TDD_TESTS,
    TDD_MOCKING,
    TDD_DEEP_MODULES,
    TDD_INTERFACE_DESIGN,
    TDD_REFACTORING,
    DIAGNOSE_OPENAI_YAML,
    DIAGNOSE_HITL_LOOP_TEMPLATE,
    ARCHITECTURE_OPENAI_YAML,
    ARCHITECTURE_LANGUAGE,
    ARCHITECTURE_INTERFACE_DESIGN,
    ARCHITECTURE_DEEPENING,
    PROTOTYPE_OPENAI_YAML,
    PROTOTYPE_LOGIC,
    PROTOTYPE_UI,
    CODEX_REVIEW_OPENAI_YAML,
    CODEX_REVIEW_HELPER,
    BRUTAL_REVIEW_OPENAI_YAML,
    HANDOFF_OPENAI_YAML,
    WORKFLOW_DOC,
    DOMAIN_DOC,
    ARTIFACT_BRIEFS_DOC,
];

const MANAGED_BUNDLE_FILES: [&str; 44] = [
    OVERVIEW_SKILL,
    OVERVIEW_OPENAI_YAML,
    CLARIFY_SKILL,
    CLARIFY_OPENAI_YAML,
    CLARIFY_ADR_FORMAT,
    CLARIFY_CONTEXT_FORMAT,
    CREATE_PRD_SKILL,
    CREATE_PRD_OPENAI_YAML,
    CREATE_PRD_FORMAT,
    PLAN_SKILL,
    PLAN_OPENAI_YAML,
    PLAN_ADR_FORMAT,
    PLAN_SUBPLAN_BRIEF,
    PLAN_GOAL_BRIEF_FORMAT,
    TDD_SKILL,
    TDD_OPENAI_YAML,
    TDD_TESTS,
    TDD_MOCKING,
    TDD_DEEP_MODULES,
    TDD_INTERFACE_DESIGN,
    TDD_REFACTORING,
    DIAGNOSE_SKILL,
    DIAGNOSE_OPENAI_YAML,
    DIAGNOSE_HITL_LOOP_TEMPLATE,
    ARCHITECTURE_SKILL,
    ARCHITECTURE_OPENAI_YAML,
    ARCHITECTURE_LANGUAGE,
    ARCHITECTURE_INTERFACE_DESIGN,
    ARCHITECTURE_DEEPENING,
    PROTOTYPE_SKILL,
    PROTOTYPE_OPENAI_YAML,
    PROTOTYPE_LOGIC,
    PROTOTYPE_UI,
    CODEX_REVIEW_SKILL,
    CODEX_REVIEW_OPENAI_YAML,
    CODEX_REVIEW_HELPER,
    BRUTAL_REVIEW_SKILL,
    BRUTAL_REVIEW_OPENAI_YAML,
    HANDOFF_SKILL,
    HANDOFF_OPENAI_YAML,
    WORKFLOW_DOC,
    DOMAIN_DOC,
    ARTIFACT_BRIEFS_DOC,
    BUNDLE_GUIDANCE,
];

const LEGACY_BUNDLE_FILES_V11: [&str; 42] = [
    OVERVIEW_SKILL,
    OVERVIEW_OPENAI_YAML,
    CLARIFY_SKILL,
    CLARIFY_OPENAI_YAML,
    CLARIFY_ADR_FORMAT,
    CLARIFY_CONTEXT_FORMAT,
    CREATE_PRD_SKILL,
    CREATE_PRD_OPENAI_YAML,
    CREATE_PRD_FORMAT,
    PLAN_SKILL,
    PLAN_OPENAI_YAML,
    PLAN_ADR_FORMAT,
    PLAN_SUBPLAN_BRIEF,
    PLAN_GOAL_BRIEF_FORMAT,
    TDD_SKILL,
    TDD_OPENAI_YAML,
    TDD_TESTS,
    TDD_MOCKING,
    TDD_DEEP_MODULES,
    TDD_INTERFACE_DESIGN,
    TDD_REFACTORING,
    DIAGNOSE_SKILL,
    DIAGNOSE_OPENAI_YAML,
    DIAGNOSE_HITL_LOOP_TEMPLATE,
    ARCHITECTURE_SKILL,
    ARCHITECTURE_OPENAI_YAML,
    ARCHITECTURE_LANGUAGE,
    ARCHITECTURE_INTERFACE_DESIGN,
    ARCHITECTURE_DEEPENING,
    PROTOTYPE_SKILL,
    PROTOTYPE_OPENAI_YAML,
    PROTOTYPE_LOGIC,
    PROTOTYPE_UI,
    CODEX_REVIEW_SKILL,
    CODEX_REVIEW_OPENAI_YAML,
    CODEX_REVIEW_HELPER,
    BRUTAL_REVIEW_SKILL,
    BRUTAL_REVIEW_OPENAI_YAML,
    WORKFLOW_DOC,
    DOMAIN_DOC,
    ARTIFACT_BRIEFS_DOC,
    BUNDLE_GUIDANCE,
];

const LEGACY_BUNDLE_FILES_V8: [&str; 40] = [
    OVERVIEW_SKILL,
    OVERVIEW_OPENAI_YAML,
    CLARIFY_SKILL,
    CLARIFY_OPENAI_YAML,
    CLARIFY_ADR_FORMAT,
    CLARIFY_CONTEXT_FORMAT,
    CREATE_PRD_SKILL,
    CREATE_PRD_OPENAI_YAML,
    CREATE_PRD_FORMAT,
    PLAN_SKILL,
    PLAN_OPENAI_YAML,
    PLAN_ADR_FORMAT,
    PLAN_SUBPLAN_BRIEF,
    PLAN_GOAL_BRIEF_FORMAT,
    TDD_SKILL,
    TDD_OPENAI_YAML,
    TDD_TESTS,
    TDD_MOCKING,
    TDD_DEEP_MODULES,
    TDD_INTERFACE_DESIGN,
    TDD_REFACTORING,
    DIAGNOSE_SKILL,
    DIAGNOSE_OPENAI_YAML,
    DIAGNOSE_HITL_LOOP_TEMPLATE,
    ARCHITECTURE_SKILL,
    ARCHITECTURE_OPENAI_YAML,
    ARCHITECTURE_LANGUAGE,
    ARCHITECTURE_INTERFACE_DESIGN,
    ARCHITECTURE_DEEPENING,
    PROTOTYPE_SKILL,
    PROTOTYPE_OPENAI_YAML,
    PROTOTYPE_LOGIC,
    PROTOTYPE_UI,
    CODEX_REVIEW_SKILL,
    CODEX_REVIEW_OPENAI_YAML,
    CODEX_REVIEW_HELPER,
    WORKFLOW_DOC,
    DOMAIN_DOC,
    ARTIFACT_BRIEFS_DOC,
    BUNDLE_GUIDANCE,
];

const LEGACY_BUNDLE_FILES_V6: [&str; 37] = [
    OVERVIEW_SKILL,
    OVERVIEW_OPENAI_YAML,
    CLARIFY_SKILL,
    CLARIFY_OPENAI_YAML,
    CLARIFY_ADR_FORMAT,
    CLARIFY_CONTEXT_FORMAT,
    CREATE_PRD_SKILL,
    CREATE_PRD_OPENAI_YAML,
    CREATE_PRD_FORMAT,
    PLAN_SKILL,
    PLAN_OPENAI_YAML,
    PLAN_ADR_FORMAT,
    PLAN_SUBPLAN_BRIEF,
    PLAN_GOAL_BRIEF_FORMAT,
    TDD_SKILL,
    TDD_OPENAI_YAML,
    TDD_TESTS,
    TDD_MOCKING,
    TDD_DEEP_MODULES,
    TDD_INTERFACE_DESIGN,
    TDD_REFACTORING,
    DIAGNOSE_SKILL,
    DIAGNOSE_OPENAI_YAML,
    DIAGNOSE_HITL_LOOP_TEMPLATE,
    ARCHITECTURE_SKILL,
    ARCHITECTURE_OPENAI_YAML,
    ARCHITECTURE_LANGUAGE,
    ARCHITECTURE_INTERFACE_DESIGN,
    ARCHITECTURE_DEEPENING,
    PROTOTYPE_SKILL,
    PROTOTYPE_OPENAI_YAML,
    PROTOTYPE_LOGIC,
    PROTOTYPE_UI,
    WORKFLOW_DOC,
    DOMAIN_DOC,
    ARTIFACT_BRIEFS_DOC,
    BUNDLE_GUIDANCE,
];

const LEGACY_BUNDLE_FILES_V5: [&str; 14] = [
    OVERVIEW_SKILL,
    CLARIFY_SKILL,
    CLARIFY_ADR_FORMAT,
    CLARIFY_CONTEXT_FORMAT,
    CREATE_PRD_SKILL,
    CREATE_PRD_FORMAT,
    PLAN_SKILL,
    PLAN_ADR_FORMAT,
    PLAN_SUBPLAN_BRIEF,
    PLAN_GOAL_BRIEF_FORMAT,
    WORKFLOW_DOC,
    DOMAIN_DOC,
    ARTIFACT_BRIEFS_DOC,
    BUNDLE_GUIDANCE,
];

const LEGACY_BUNDLE_FILES_V4: [&str; 14] = [
    OVERVIEW_SKILL,
    CLARIFY_SKILL,
    CLARIFY_ADR_FORMAT,
    CLARIFY_CONTEXT_FORMAT,
    CREATE_PRD_SKILL,
    CREATE_PRD_FORMAT,
    PLAN_SKILL,
    PLAN_ADR_FORMAT,
    PLAN_SUBPLAN_BRIEF,
    LEGACY_PLAN_EXECUTION_PROMPT_FORMAT,
    WORKFLOW_DOC,
    DOMAIN_DOC,
    ARTIFACT_BRIEFS_DOC,
    BUNDLE_GUIDANCE,
];

const LEGACY_BUNDLE_FILES_V3: [&str; 8] = [
    OVERVIEW_SKILL,
    CLARIFY_SKILL,
    CREATE_PRD_SKILL,
    PLAN_SKILL,
    WORKFLOW_DOC,
    DOMAIN_DOC,
    ARTIFACT_BRIEFS_DOC,
    BUNDLE_GUIDANCE,
];

const LEGACY_BUNDLE_FILES_V2: [&str; 5] = [
    OVERVIEW_SKILL,
    CLARIFY_SKILL,
    CREATE_PRD_SKILL,
    PLAN_SKILL,
    BUNDLE_GUIDANCE,
];

const LEGACY_BUNDLE_FILES_V1: [&str; 2] = [OVERVIEW_SKILL, BUNDLE_GUIDANCE];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct BundleMarker {
    pub managed_by: String,
    pub version: u32,
    pub files: Vec<String>,
}

pub(super) fn managed_skill_files() -> &'static [&'static str] {
    &MANAGED_SKILL_FILES
}

pub(super) fn managed_supporting_doc_files() -> &'static [&'static str] {
    &MANAGED_SUPPORTING_DOC_FILES
}

pub(super) fn current_bundle_files() -> Vec<String> {
    files_to_strings(&MANAGED_BUNDLE_FILES)
}

pub(super) fn is_current_bundle_file(relative: &str) -> bool {
    MANAGED_BUNDLE_FILES.contains(&relative)
}

pub(super) fn managed_restore_files() -> Vec<&'static str> {
    let mut files = MANAGED_BUNDLE_FILES.to_vec();
    files.push(LEGACY_PLAN_EXECUTION_PROMPT_FORMAT);
    files.push(BUNDLE_MARKER);
    files
}

pub(super) fn expected_current_body(relative: &str) -> Result<String> {
    expected_body(relative).ok_or_else(|| {
        Codex1Error::SetupBundle(format!(
            "missing expected body for managed setup file {relative}"
        ))
    })
}

pub(super) fn expected_body(relative: &str) -> Option<String> {
    Some(
        match relative {
            OVERVIEW_SKILL => include_str!("../../.agents/skills/codex1/SKILL.md"),
            OVERVIEW_OPENAI_YAML => {
                include_str!("../../.agents/skills/codex1/agents/openai.yaml")
            }
            CLARIFY_SKILL => include_str!("../../.agents/skills/clarify/SKILL.md"),
            CLARIFY_OPENAI_YAML => {
                include_str!("../../.agents/skills/clarify/agents/openai.yaml")
            }
            CLARIFY_ADR_FORMAT => include_str!("../../.agents/skills/clarify/ADR-FORMAT.md"),
            CLARIFY_CONTEXT_FORMAT => {
                include_str!("../../.agents/skills/clarify/CONTEXT-FORMAT.md")
            }
            CREATE_PRD_SKILL => include_str!("../../.agents/skills/create-prd/SKILL.md"),
            CREATE_PRD_OPENAI_YAML => {
                include_str!("../../.agents/skills/create-prd/agents/openai.yaml")
            }
            CREATE_PRD_FORMAT => include_str!("../../.agents/skills/create-prd/PRD-FORMAT.md"),
            PLAN_SKILL => include_str!("../../.agents/skills/plan/SKILL.md"),
            PLAN_OPENAI_YAML => include_str!("../../.agents/skills/plan/agents/openai.yaml"),
            PLAN_ADR_FORMAT => include_str!("../../.agents/skills/plan/ADR-FORMAT.md"),
            PLAN_SUBPLAN_BRIEF => include_str!("../../.agents/skills/plan/SUBPLAN-BRIEF.md"),
            PLAN_GOAL_BRIEF_FORMAT => {
                include_str!("../../.agents/skills/plan/GOAL-BRIEF-FORMAT.md")
            }
            TDD_SKILL => include_str!("../../.agents/skills/tdd/SKILL.md"),
            TDD_OPENAI_YAML => include_str!("../../.agents/skills/tdd/agents/openai.yaml"),
            TDD_TESTS => include_str!("../../.agents/skills/tdd/tests.md"),
            TDD_MOCKING => include_str!("../../.agents/skills/tdd/mocking.md"),
            TDD_DEEP_MODULES => include_str!("../../.agents/skills/tdd/deep-modules.md"),
            TDD_INTERFACE_DESIGN => include_str!("../../.agents/skills/tdd/interface-design.md"),
            TDD_REFACTORING => include_str!("../../.agents/skills/tdd/refactoring.md"),
            DIAGNOSE_SKILL => include_str!("../../.agents/skills/diagnose/SKILL.md"),
            DIAGNOSE_OPENAI_YAML => {
                include_str!("../../.agents/skills/diagnose/agents/openai.yaml")
            }
            DIAGNOSE_HITL_LOOP_TEMPLATE => {
                include_str!("../../.agents/skills/diagnose/scripts/hitl-loop.template.sh")
            }
            ARCHITECTURE_SKILL => {
                include_str!("../../.agents/skills/improve-codebase-architecture/SKILL.md")
            }
            ARCHITECTURE_OPENAI_YAML => include_str!(
                "../../.agents/skills/improve-codebase-architecture/agents/openai.yaml"
            ),
            ARCHITECTURE_LANGUAGE => {
                include_str!("../../.agents/skills/improve-codebase-architecture/LANGUAGE.md")
            }
            ARCHITECTURE_INTERFACE_DESIGN => include_str!(
                "../../.agents/skills/improve-codebase-architecture/INTERFACE-DESIGN.md"
            ),
            ARCHITECTURE_DEEPENING => {
                include_str!("../../.agents/skills/improve-codebase-architecture/DEEPENING.md")
            }
            PROTOTYPE_SKILL => include_str!("../../.agents/skills/prototype/SKILL.md"),
            PROTOTYPE_OPENAI_YAML => {
                include_str!("../../.agents/skills/prototype/agents/openai.yaml")
            }
            PROTOTYPE_LOGIC => include_str!("../../.agents/skills/prototype/LOGIC.md"),
            PROTOTYPE_UI => include_str!("../../.agents/skills/prototype/UI.md"),
            CODEX_REVIEW_SKILL => include_str!("../../.agents/skills/codex-review/SKILL.md"),
            CODEX_REVIEW_OPENAI_YAML => {
                include_str!("../../.agents/skills/codex-review/agents/openai.yaml")
            }
            CODEX_REVIEW_HELPER => {
                include_str!("../../.agents/skills/codex-review/scripts/codex-review")
            }
            BRUTAL_REVIEW_SKILL => include_str!("../../.agents/skills/brutal-review/SKILL.md"),
            BRUTAL_REVIEW_OPENAI_YAML => {
                include_str!("../../.agents/skills/brutal-review/agents/openai.yaml")
            }
            HANDOFF_SKILL => include_str!("../../.agents/skills/handoff/SKILL.md"),
            HANDOFF_OPENAI_YAML => include_str!("../../.agents/skills/handoff/agents/openai.yaml"),
            WORKFLOW_DOC => include_str!("../../docs/agents/codex1-workflow.md"),
            DOMAIN_DOC => include_str!("../../docs/agents/codex1-domain.md"),
            ARTIFACT_BRIEFS_DOC => include_str!("../../docs/agents/codex1-artifact-briefs.md"),
            LEGACY_PLAN_EXECUTION_PROMPT_FORMAT => legacy_execution_prompt_format_body(),
            BUNDLE_GUIDANCE => guidance::body(),
            BUNDLE_MARKER => return Some(bundle_marker_body()),
            _ => return None,
        }
        .to_string(),
    )
}

pub(super) fn is_current_marker(marker: &BundleMarker) -> bool {
    marker.managed_by == "codex1-managed"
        && marker.version == BUNDLE_VERSION
        && marker.files == current_bundle_files()
}

pub(super) fn is_managed_bundle_marker(marker: &BundleMarker) -> bool {
    marker.managed_by == "codex1-managed"
        && legacy_bundle_file_sets()
            .iter()
            .any(|files| marker.files == files_to_strings(files))
}

pub(super) fn marker_allows_file_repair(marker: Option<&BundleMarker>, relative: &str) -> bool {
    marker.is_some_and(|marker| {
        is_managed_bundle_marker(marker) && marker.files.iter().any(|file| file == relative)
    })
}

pub(super) fn bundle_marker_body() -> String {
    serde_json::to_string_pretty(&BundleMarker {
        managed_by: "codex1-managed".into(),
        version: BUNDLE_VERSION,
        files: current_bundle_files(),
    })
    .unwrap()
        + "\n"
}

pub(super) fn is_managed_restore_body(relative: &str, text: &str) -> bool {
    if relative == BUNDLE_MARKER {
        return serde_json::from_str::<BundleMarker>(text)
            .is_ok_and(|marker| is_managed_bundle_marker(&marker));
    }
    expected_body(relative).as_deref() == Some(text) || matches_legacy_managed_body(relative, text)
}

fn legacy_bundle_file_sets() -> [&'static [&'static str]; 9] {
    [
        &MANAGED_BUNDLE_FILES,
        &LEGACY_BUNDLE_FILES_V11,
        &LEGACY_BUNDLE_FILES_V8,
        &LEGACY_BUNDLE_FILES_V6,
        &LEGACY_BUNDLE_FILES_V5,
        &LEGACY_BUNDLE_FILES_V4,
        &LEGACY_BUNDLE_FILES_V3,
        &LEGACY_BUNDLE_FILES_V2,
        &LEGACY_BUNDLE_FILES_V1,
    ]
}

fn files_to_strings(files: &[&str]) -> Vec<String> {
    files.iter().map(|file| (*file).to_string()).collect()
}

struct LegacyBodyFingerprint {
    relative: &'static str,
    len: usize,
    fnv1a64: u64,
}

const LEGACY_MANAGED_BODY_FINGERPRINTS: [LegacyBodyFingerprint; 12] = [
    LegacyBodyFingerprint {
        relative: OVERVIEW_SKILL,
        len: 2431,
        fnv1a64: 0x5bdad4b242679346,
    },
    LegacyBodyFingerprint {
        relative: CLARIFY_SKILL,
        len: 3919,
        fnv1a64: 0xb75e2bb26bbfe162,
    },
    LegacyBodyFingerprint {
        relative: CREATE_PRD_SKILL,
        len: 3221,
        fnv1a64: 0xe77c78c652529d1e,
    },
    LegacyBodyFingerprint {
        relative: CREATE_PRD_FORMAT,
        len: 1834,
        fnv1a64: 0x0c53c184ad841164,
    },
    LegacyBodyFingerprint {
        relative: PLAN_SKILL,
        len: 6599,
        fnv1a64: 0x59643282b16f9eff,
    },
    LegacyBodyFingerprint {
        relative: PLAN_GOAL_BRIEF_FORMAT,
        len: 1818,
        fnv1a64: 0x2bf7a05a412c9df7,
    },
    LegacyBodyFingerprint {
        relative: CODEX_REVIEW_SKILL,
        len: 6209,
        fnv1a64: 0xebc17dcf5b43258a,
    },
    LegacyBodyFingerprint {
        relative: CODEX_REVIEW_SKILL,
        len: 5325,
        fnv1a64: 0xcfe694510fae4fe2,
    },
    LegacyBodyFingerprint {
        relative: CODEX_REVIEW_HELPER,
        len: 14947,
        fnv1a64: 0x3b8b15d2cfecf630,
    },
    LegacyBodyFingerprint {
        relative: CODEX_REVIEW_HELPER,
        len: 6560,
        fnv1a64: 0x1eb8f1e0bf4d22d1,
    },
    LegacyBodyFingerprint {
        relative: WORKFLOW_DOC,
        len: 1904,
        fnv1a64: 0x576d42530da8f540,
    },
    LegacyBodyFingerprint {
        relative: ARTIFACT_BRIEFS_DOC,
        len: 2225,
        fnv1a64: 0x1defde1b457d9232,
    },
];

fn matches_legacy_managed_body(relative: &str, text: &str) -> bool {
    LEGACY_MANAGED_BODY_FINGERPRINTS.iter().any(|fingerprint| {
        fingerprint.relative == relative
            && fingerprint.len == text.len()
            && fingerprint.fnv1a64 == fnv1a64(text.as_bytes())
    })
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn legacy_execution_prompt_format_body() -> &'static str {
    r#"# Execution Prompt Format

`EXECUTION_PROMPT.md` contains the objective text the user copies after typing native `/goal`.

It is a copy source. The prompt must not tell Codex to read `EXECUTION_PROMPT.md`; the user has already copied from it.

## Native Goal Objective Must Include

- Mission path
- Primary artifacts to read
- Execution order
- Subplan selection rules
- Worker/subagent rules when useful
- Editable scope
- Proof recording rules
- Review and triage rules
- Explicit completion criteria
- If completion cannot be reached
- Closeout rules
- Prohibited actions

## Completion Criteria

Completion criteria are only completion criteria. Do not include pause, escalation, "ask the user", or "wait for clarification" criteria.

Good completion criteria are observable:

- Required ready subplans are complete or explicitly triaged not applicable.
- Required proofs exist and were audited.
- PRD success criteria are satisfied or recorded as deferred with reason.
- Closeout summarizes completed, superseded, paused, deferred, and risky work.

## No-question Execution

The `/goal` execution phase may not ask questions. If artifacts are insufficient, Codex should record non-completion evidence, blockers, accepted risks, or deferred work rather than inventing scope or asking the user.

## Worker Rules

When using workers, give each worker explicit ownership, relevant artifacts, editable scope, proof expectations, and non-goals. Workers should not edit mission-level artifacts unless assigned.

## Prohibited Actions

Always prohibit:

- Creating, inspecting, or completing native goal state from Codex1.
- Treating `codex1 inspect`, setup status, events, or receipts as completion proof.
- Reading `EXECUTION_PROMPT.md` as the first step of the pasted objective.
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn current_bundle_files_have_no_duplicates() {
        let mut seen = HashSet::new();
        for file in MANAGED_BUNDLE_FILES {
            assert!(seen.insert(file), "duplicate managed setup file: {file}");
        }
    }

    #[test]
    fn skills_and_supporting_docs_are_in_the_current_bundle() {
        for file in MANAGED_SKILL_FILES
            .iter()
            .chain(MANAGED_SUPPORTING_DOC_FILES.iter())
        {
            assert!(MANAGED_BUNDLE_FILES.contains(file), "{file}");
        }
    }

    #[test]
    fn current_managed_files_have_expected_bodies() {
        for relative in MANAGED_BUNDLE_FILES {
            assert!(expected_body(relative).is_some(), "{relative}");
        }
    }

    #[test]
    fn marker_body_matches_expected_files() {
        let marker: BundleMarker = serde_json::from_str(&bundle_marker_body()).unwrap();
        assert!(is_current_marker(&marker));
        assert_eq!(marker.version, BUNDLE_VERSION);
        assert_eq!(marker.files, current_bundle_files());
    }

    #[test]
    fn checked_in_marker_matches_generated_marker() {
        assert_eq!(
            include_str!("../../.codex1/setup-bundle.json"),
            bundle_marker_body()
        );
    }

    #[test]
    fn generated_managed_skill_bodies_match_checked_in_files() {
        for relative in MANAGED_SKILL_FILES {
            let checked_in = std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative),
            )
            .unwrap();
            assert_eq!(expected_body(relative).unwrap(), checked_in, "{relative}");
        }
    }

    #[test]
    fn legacy_fingerprints_cover_recent_managed_body_changes() {
        for (relative, len, fnv1a64) in [
            (CLARIFY_SKILL, 3919, 0xb75e2bb26bbfe162),
            (CREATE_PRD_SKILL, 3221, 0xe77c78c652529d1e),
            (CREATE_PRD_FORMAT, 1834, 0x0c53c184ad841164),
            (ARTIFACT_BRIEFS_DOC, 2225, 0x1defde1b457d9232),
            (CODEX_REVIEW_SKILL, 6209, 0xebc17dcf5b43258a),
            (CODEX_REVIEW_HELPER, 14947, 0x3b8b15d2cfecf630),
        ] {
            assert!(
                LEGACY_MANAGED_BODY_FINGERPRINTS.iter().any(|fingerprint| {
                    fingerprint.relative == relative
                        && fingerprint.len == len
                        && fingerprint.fnv1a64 == fnv1a64
                }),
                "missing legacy managed body fingerprint for {relative}"
            );
        }
    }
}
