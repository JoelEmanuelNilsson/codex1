use serde::{Deserialize, Serialize};

use super::guidance;

pub(super) const BUNDLE_VERSION: u32 = 15;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BundleFileRole {
    ManagedSkill,
    SupportingDoc,
    Guidance,
}

impl BundleFileRole {
    pub(super) fn materialize_reason(self) -> &'static str {
        match self {
            Self::ManagedSkill => "managed skill",
            Self::SupportingDoc => "managed supporting doc",
            Self::Guidance => "managed guidance",
        }
    }

    fn is_owned_file(self) -> bool {
        matches!(self, Self::ManagedSkill | Self::SupportingDoc)
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct BundleEntry {
    pub(super) relative: &'static str,
    pub(super) role: BundleFileRole,
    body: BundleBody,
}

impl BundleEntry {
    const fn managed_skill(relative: &'static str, body: &'static str) -> Self {
        Self {
            relative,
            role: BundleFileRole::ManagedSkill,
            body: BundleBody::Static(body),
        }
    }

    const fn supporting_doc(relative: &'static str, body: &'static str) -> Self {
        Self {
            relative,
            role: BundleFileRole::SupportingDoc,
            body: BundleBody::Static(body),
        }
    }

    const fn guidance(relative: &'static str) -> Self {
        Self {
            relative,
            role: BundleFileRole::Guidance,
            body: BundleBody::Guidance,
        }
    }

    pub(super) fn expected_body(self) -> String {
        match self.body {
            BundleBody::Static(body) => body.to_string(),
            BundleBody::Guidance => guidance::body().to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum BundleBody {
    Static(&'static str),
    Guidance,
}

const CURRENT_BUNDLE_ENTRIES: [BundleEntry; 31] = [
    BundleEntry::managed_skill(
        CLARIFY_SKILL,
        include_str!("../../.agents/skills/clarify/SKILL.md"),
    ),
    BundleEntry::supporting_doc(
        CLARIFY_OPENAI_YAML,
        include_str!("../../.agents/skills/clarify/agents/openai.yaml"),
    ),
    BundleEntry::supporting_doc(
        CLARIFY_ADR_FORMAT,
        include_str!("../../.agents/skills/clarify/ADR-FORMAT.md"),
    ),
    BundleEntry::supporting_doc(
        CLARIFY_CONTEXT_FORMAT,
        include_str!("../../.agents/skills/clarify/CONTEXT-FORMAT.md"),
    ),
    BundleEntry::managed_skill(
        CREATE_PRD_SKILL,
        include_str!("../../.agents/skills/create-prd/SKILL.md"),
    ),
    BundleEntry::supporting_doc(
        CREATE_PRD_OPENAI_YAML,
        include_str!("../../.agents/skills/create-prd/agents/openai.yaml"),
    ),
    BundleEntry::supporting_doc(
        CREATE_PRD_FORMAT,
        include_str!("../../.agents/skills/create-prd/PRD-FORMAT.md"),
    ),
    BundleEntry::managed_skill(TDD_SKILL, include_str!("../../.agents/skills/tdd/SKILL.md")),
    BundleEntry::supporting_doc(
        TDD_OPENAI_YAML,
        include_str!("../../.agents/skills/tdd/agents/openai.yaml"),
    ),
    BundleEntry::supporting_doc(TDD_TESTS, include_str!("../../.agents/skills/tdd/tests.md")),
    BundleEntry::supporting_doc(
        TDD_MOCKING,
        include_str!("../../.agents/skills/tdd/mocking.md"),
    ),
    BundleEntry::supporting_doc(
        TDD_DEEP_MODULES,
        include_str!("../../.agents/skills/tdd/deep-modules.md"),
    ),
    BundleEntry::supporting_doc(
        TDD_INTERFACE_DESIGN,
        include_str!("../../.agents/skills/tdd/interface-design.md"),
    ),
    BundleEntry::supporting_doc(
        TDD_REFACTORING,
        include_str!("../../.agents/skills/tdd/refactoring.md"),
    ),
    BundleEntry::managed_skill(
        DIAGNOSE_SKILL,
        include_str!("../../.agents/skills/diagnose/SKILL.md"),
    ),
    BundleEntry::supporting_doc(
        DIAGNOSE_OPENAI_YAML,
        include_str!("../../.agents/skills/diagnose/agents/openai.yaml"),
    ),
    BundleEntry::supporting_doc(
        DIAGNOSE_HITL_LOOP_TEMPLATE,
        include_str!("../../.agents/skills/diagnose/scripts/hitl-loop.template.sh"),
    ),
    BundleEntry::managed_skill(
        ARCHITECTURE_SKILL,
        include_str!("../../.agents/skills/improve-codebase-architecture/SKILL.md"),
    ),
    BundleEntry::supporting_doc(
        ARCHITECTURE_OPENAI_YAML,
        include_str!("../../.agents/skills/improve-codebase-architecture/agents/openai.yaml"),
    ),
    BundleEntry::supporting_doc(
        ARCHITECTURE_LANGUAGE,
        include_str!("../../.agents/skills/improve-codebase-architecture/LANGUAGE.md"),
    ),
    BundleEntry::supporting_doc(
        ARCHITECTURE_INTERFACE_DESIGN,
        include_str!("../../.agents/skills/improve-codebase-architecture/INTERFACE-DESIGN.md"),
    ),
    BundleEntry::supporting_doc(
        ARCHITECTURE_DEEPENING,
        include_str!("../../.agents/skills/improve-codebase-architecture/DEEPENING.md"),
    ),
    BundleEntry::managed_skill(
        CODEX_REVIEW_SKILL,
        include_str!("../../.agents/skills/codex-review/SKILL.md"),
    ),
    BundleEntry::supporting_doc(
        CODEX_REVIEW_OPENAI_YAML,
        include_str!("../../.agents/skills/codex-review/agents/openai.yaml"),
    ),
    BundleEntry::supporting_doc(
        CODEX_REVIEW_HELPER,
        include_str!("../../.agents/skills/codex-review/scripts/codex-review"),
    ),
    BundleEntry::managed_skill(
        HANDOFF_SKILL,
        include_str!("../../.agents/skills/handoff/SKILL.md"),
    ),
    BundleEntry::supporting_doc(
        HANDOFF_OPENAI_YAML,
        include_str!("../../.agents/skills/handoff/agents/openai.yaml"),
    ),
    BundleEntry::supporting_doc(
        WORKFLOW_DOC,
        include_str!("../../docs/agents/codex1-workflow.md"),
    ),
    BundleEntry::supporting_doc(
        DOMAIN_DOC,
        include_str!("../../docs/agents/codex1-domain.md"),
    ),
    BundleEntry::supporting_doc(
        ARTIFACT_BRIEFS_DOC,
        include_str!("../../docs/agents/codex1-artifact-briefs.md"),
    ),
    BundleEntry::guidance(BUNDLE_GUIDANCE),
];

#[derive(Clone, Copy, Debug)]
struct LegacyReleaseSpec {
    workflow_skills: bool,
    openai_yaml: bool,
    clarify_formats: bool,
    create_prd_format: bool,
    plan_adr_format: bool,
    plan_subplan_brief: bool,
    plan_goal_brief_format: bool,
    plan_legacy_execution_prompt: bool,
    tdd: bool,
    diagnose: bool,
    architecture: bool,
    prototype: bool,
    codex_review: bool,
    brutal_review: bool,
    handoff: bool,
    agent_docs: bool,
}

const LEGACY_RELEASES: [LegacyReleaseSpec; 9] = [
    LegacyReleaseSpec {
        workflow_skills: true,
        openai_yaml: true,
        clarify_formats: true,
        create_prd_format: true,
        plan_adr_format: true,
        plan_subplan_brief: true,
        plan_goal_brief_format: true,
        plan_legacy_execution_prompt: false,
        tdd: true,
        diagnose: true,
        architecture: true,
        prototype: true,
        codex_review: true,
        brutal_review: true,
        handoff: true,
        agent_docs: true,
    },
    LegacyReleaseSpec {
        workflow_skills: true,
        openai_yaml: true,
        clarify_formats: true,
        create_prd_format: true,
        plan_adr_format: true,
        plan_subplan_brief: true,
        plan_goal_brief_format: true,
        plan_legacy_execution_prompt: false,
        tdd: true,
        diagnose: true,
        architecture: true,
        prototype: true,
        codex_review: true,
        brutal_review: true,
        handoff: false,
        agent_docs: true,
    },
    LegacyReleaseSpec {
        workflow_skills: true,
        openai_yaml: true,
        clarify_formats: true,
        create_prd_format: true,
        plan_adr_format: true,
        plan_subplan_brief: true,
        plan_goal_brief_format: true,
        plan_legacy_execution_prompt: false,
        tdd: true,
        diagnose: true,
        architecture: true,
        prototype: true,
        codex_review: true,
        brutal_review: false,
        handoff: false,
        agent_docs: true,
    },
    LegacyReleaseSpec {
        workflow_skills: true,
        openai_yaml: true,
        clarify_formats: true,
        create_prd_format: true,
        plan_adr_format: true,
        plan_subplan_brief: true,
        plan_goal_brief_format: true,
        plan_legacy_execution_prompt: false,
        tdd: true,
        diagnose: true,
        architecture: true,
        prototype: true,
        codex_review: false,
        brutal_review: false,
        handoff: false,
        agent_docs: true,
    },
    LegacyReleaseSpec {
        workflow_skills: true,
        openai_yaml: false,
        clarify_formats: true,
        create_prd_format: true,
        plan_adr_format: true,
        plan_subplan_brief: true,
        plan_goal_brief_format: true,
        plan_legacy_execution_prompt: false,
        tdd: false,
        diagnose: false,
        architecture: false,
        prototype: false,
        codex_review: false,
        brutal_review: false,
        handoff: false,
        agent_docs: true,
    },
    LegacyReleaseSpec {
        workflow_skills: true,
        openai_yaml: false,
        clarify_formats: true,
        create_prd_format: true,
        plan_adr_format: true,
        plan_subplan_brief: true,
        plan_goal_brief_format: false,
        plan_legacy_execution_prompt: true,
        tdd: false,
        diagnose: false,
        architecture: false,
        prototype: false,
        codex_review: false,
        brutal_review: false,
        handoff: false,
        agent_docs: true,
    },
    LegacyReleaseSpec {
        workflow_skills: true,
        openai_yaml: false,
        clarify_formats: false,
        create_prd_format: false,
        plan_adr_format: false,
        plan_subplan_brief: false,
        plan_goal_brief_format: false,
        plan_legacy_execution_prompt: false,
        tdd: false,
        diagnose: false,
        architecture: false,
        prototype: false,
        codex_review: false,
        brutal_review: false,
        handoff: false,
        agent_docs: true,
    },
    LegacyReleaseSpec {
        workflow_skills: true,
        openai_yaml: false,
        clarify_formats: false,
        create_prd_format: false,
        plan_adr_format: false,
        plan_subplan_brief: false,
        plan_goal_brief_format: false,
        plan_legacy_execution_prompt: false,
        tdd: false,
        diagnose: false,
        architecture: false,
        prototype: false,
        codex_review: false,
        brutal_review: false,
        handoff: false,
        agent_docs: false,
    },
    LegacyReleaseSpec {
        workflow_skills: false,
        openai_yaml: false,
        clarify_formats: false,
        create_prd_format: false,
        plan_adr_format: false,
        plan_subplan_brief: false,
        plan_goal_brief_format: false,
        plan_legacy_execution_prompt: false,
        tdd: false,
        diagnose: false,
        architecture: false,
        prototype: false,
        codex_review: false,
        brutal_review: false,
        handoff: false,
        agent_docs: false,
    },
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct BundleMarker {
    pub managed_by: String,
    pub version: u32,
    pub files: Vec<String>,
}

pub(super) fn entries_by_role(role: BundleFileRole) -> impl Iterator<Item = &'static BundleEntry> {
    CURRENT_BUNDLE_ENTRIES
        .iter()
        .filter(move |entry| entry.role == role)
}

pub(super) fn owned_file_entries() -> impl Iterator<Item = &'static BundleEntry> {
    CURRENT_BUNDLE_ENTRIES
        .iter()
        .filter(|entry| entry.role.is_owned_file())
}

pub(super) fn current_bundle_files() -> Vec<String> {
    CURRENT_BUNDLE_ENTRIES
        .iter()
        .map(|entry| entry.relative.to_string())
        .collect()
}

pub(super) fn is_current_bundle_file(relative: &str) -> bool {
    current_entry(relative).is_some()
}

pub(super) fn managed_restore_files() -> Vec<&'static str> {
    let mut files: Vec<_> = CURRENT_BUNDLE_ENTRIES
        .iter()
        .map(|entry| entry.relative)
        .collect();
    for retired in retired_legacy_files() {
        if !files.contains(&retired) {
            files.push(retired);
        }
    }
    files.push(BUNDLE_MARKER);
    files
}

pub(super) fn expected_body(relative: &str) -> Option<String> {
    if relative == BUNDLE_MARKER {
        return Some(bundle_marker_body());
    }
    if relative == LEGACY_PLAN_EXECUTION_PROMPT_FORMAT {
        return Some(legacy_execution_prompt_format_body().to_string());
    }
    current_entry(relative).map(|entry| entry.expected_body())
}

pub(super) fn is_current_marker(marker: &BundleMarker) -> bool {
    marker.managed_by == "codex1-managed"
        && marker.version == BUNDLE_VERSION
        && marker.files == current_bundle_files()
}

pub(super) fn is_managed_bundle_marker(marker: &BundleMarker) -> bool {
    marker.managed_by == "codex1-managed" && files_match_known_bundle(&marker.files)
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

fn current_entry(relative: &str) -> Option<&'static BundleEntry> {
    CURRENT_BUNDLE_ENTRIES
        .iter()
        .find(|entry| entry.relative == relative)
}

fn files_match_known_bundle(files: &[String]) -> bool {
    files == current_bundle_files()
        || files_match(files, &pre_plan_retirement_bundle_files())
        || LEGACY_RELEASES
            .iter()
            .any(|release| files_match(files, &legacy_bundle_files(*release)))
}

fn files_match(actual: &[String], expected: &[&str]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected.iter())
            .all(|(actual, expected)| actual == expected)
}

fn legacy_bundle_files(release: LegacyReleaseSpec) -> Vec<&'static str> {
    let mut files = vec![OVERVIEW_SKILL];
    if release.openai_yaml {
        files.push(OVERVIEW_OPENAI_YAML);
    }
    if release.workflow_skills {
        push_workflow_files(&mut files, release);
    }
    if release.tdd {
        push_tdd_files(&mut files, release.openai_yaml);
    }
    if release.diagnose {
        push_diagnose_files(&mut files, release.openai_yaml);
    }
    if release.architecture {
        push_architecture_files(&mut files, release.openai_yaml);
    }
    if release.prototype {
        push_prototype_files(&mut files, release.openai_yaml);
    }
    if release.codex_review {
        push_codex_review_files(&mut files, release.openai_yaml);
    }
    if release.brutal_review {
        push_brutal_review_files(&mut files, release.openai_yaml);
    }
    if release.handoff {
        files.push(HANDOFF_SKILL);
        if release.openai_yaml {
            files.push(HANDOFF_OPENAI_YAML);
        }
    }
    if release.agent_docs {
        files.extend([WORKFLOW_DOC, DOMAIN_DOC, ARTIFACT_BRIEFS_DOC]);
    }
    files.push(BUNDLE_GUIDANCE);
    files
}

fn pre_plan_retirement_bundle_files() -> Vec<&'static str> {
    let release = LEGACY_RELEASES[0];
    let mut files = Vec::new();
    push_workflow_files(&mut files, release);
    push_tdd_files(&mut files, release.openai_yaml);
    push_diagnose_files(&mut files, release.openai_yaml);
    push_architecture_files(&mut files, release.openai_yaml);
    push_codex_review_files(&mut files, release.openai_yaml);
    files.push(HANDOFF_SKILL);
    files.push(HANDOFF_OPENAI_YAML);
    files.extend([WORKFLOW_DOC, DOMAIN_DOC, ARTIFACT_BRIEFS_DOC]);
    files.push(BUNDLE_GUIDANCE);
    files
}

fn push_workflow_files(files: &mut Vec<&'static str>, release: LegacyReleaseSpec) {
    files.push(CLARIFY_SKILL);
    if release.openai_yaml {
        files.push(CLARIFY_OPENAI_YAML);
    }
    if release.clarify_formats {
        files.extend([CLARIFY_ADR_FORMAT, CLARIFY_CONTEXT_FORMAT]);
    }
    files.push(CREATE_PRD_SKILL);
    if release.openai_yaml {
        files.push(CREATE_PRD_OPENAI_YAML);
    }
    if release.create_prd_format {
        files.push(CREATE_PRD_FORMAT);
    }
    files.push(PLAN_SKILL);
    if release.openai_yaml {
        files.push(PLAN_OPENAI_YAML);
    }
    if release.plan_adr_format {
        files.push(PLAN_ADR_FORMAT);
    }
    if release.plan_subplan_brief {
        files.push(PLAN_SUBPLAN_BRIEF);
    }
    if release.plan_goal_brief_format {
        files.push(PLAN_GOAL_BRIEF_FORMAT);
    }
    if release.plan_legacy_execution_prompt {
        files.push(LEGACY_PLAN_EXECUTION_PROMPT_FORMAT);
    }
}

fn push_tdd_files(files: &mut Vec<&'static str>, openai_yaml: bool) {
    files.push(TDD_SKILL);
    if openai_yaml {
        files.push(TDD_OPENAI_YAML);
    }
    files.extend([
        TDD_TESTS,
        TDD_MOCKING,
        TDD_DEEP_MODULES,
        TDD_INTERFACE_DESIGN,
        TDD_REFACTORING,
    ]);
}

fn push_diagnose_files(files: &mut Vec<&'static str>, openai_yaml: bool) {
    files.push(DIAGNOSE_SKILL);
    if openai_yaml {
        files.push(DIAGNOSE_OPENAI_YAML);
    }
    files.push(DIAGNOSE_HITL_LOOP_TEMPLATE);
}

fn push_architecture_files(files: &mut Vec<&'static str>, openai_yaml: bool) {
    files.push(ARCHITECTURE_SKILL);
    if openai_yaml {
        files.push(ARCHITECTURE_OPENAI_YAML);
    }
    files.extend([
        ARCHITECTURE_LANGUAGE,
        ARCHITECTURE_INTERFACE_DESIGN,
        ARCHITECTURE_DEEPENING,
    ]);
}

fn push_prototype_files(files: &mut Vec<&'static str>, openai_yaml: bool) {
    files.push(PROTOTYPE_SKILL);
    if openai_yaml {
        files.push(PROTOTYPE_OPENAI_YAML);
    }
    files.extend([PROTOTYPE_LOGIC, PROTOTYPE_UI]);
}

fn push_codex_review_files(files: &mut Vec<&'static str>, openai_yaml: bool) {
    files.push(CODEX_REVIEW_SKILL);
    if openai_yaml {
        files.push(CODEX_REVIEW_OPENAI_YAML);
    }
    files.push(CODEX_REVIEW_HELPER);
}

fn push_brutal_review_files(files: &mut Vec<&'static str>, openai_yaml: bool) {
    files.push(BRUTAL_REVIEW_SKILL);
    if openai_yaml {
        files.push(BRUTAL_REVIEW_OPENAI_YAML);
    }
}

fn retired_legacy_files() -> Vec<&'static str> {
    let mut retired = Vec::new();
    for relative in pre_plan_retirement_bundle_files() {
        if !is_current_bundle_file(relative) && !retired.contains(&relative) {
            retired.push(relative);
        }
    }
    for release in LEGACY_RELEASES {
        for relative in legacy_bundle_files(release) {
            if !is_current_bundle_file(relative) && !retired.contains(&relative) {
                retired.push(relative);
            }
        }
    }
    retired
}

#[cfg(test)]
fn files_to_strings(files: &[&str]) -> Vec<String> {
    files.iter().map(|file| (*file).to_string()).collect()
}

struct LegacyBodyFingerprint {
    relative: &'static str,
    len: usize,
    fnv1a64: u64,
}

const LEGACY_MANAGED_BODY_FINGERPRINTS: [LegacyBodyFingerprint; 25] = [
    LegacyBodyFingerprint {
        relative: OVERVIEW_SKILL,
        len: 3209,
        fnv1a64: 0x667b264ffb0259ce,
    },
    LegacyBodyFingerprint {
        relative: OVERVIEW_OPENAI_YAML,
        len: 170,
        fnv1a64: 0x6d68d149503a1d9f,
    },
    LegacyBodyFingerprint {
        relative: PROTOTYPE_SKILL,
        len: 3582,
        fnv1a64: 0x93f2aa6e2fd036d8,
    },
    LegacyBodyFingerprint {
        relative: PROTOTYPE_OPENAI_YAML,
        len: 198,
        fnv1a64: 0x8cbd7c6c5cacca56,
    },
    LegacyBodyFingerprint {
        relative: PROTOTYPE_LOGIC,
        len: 5560,
        fnv1a64: 0xd90b2b8bae3e3186,
    },
    LegacyBodyFingerprint {
        relative: PROTOTYPE_UI,
        len: 6751,
        fnv1a64: 0x95c3d45f92e30a0b,
    },
    LegacyBodyFingerprint {
        relative: BRUTAL_REVIEW_SKILL,
        len: 7003,
        fnv1a64: 0xe6befb5de5631afb,
    },
    LegacyBodyFingerprint {
        relative: BRUTAL_REVIEW_OPENAI_YAML,
        len: 201,
        fnv1a64: 0x273260d7035b709b,
    },
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
        relative: PLAN_SKILL,
        len: 12566,
        fnv1a64: 0xf0bb154ce15a5f52,
    },
    LegacyBodyFingerprint {
        relative: PLAN_OPENAI_YAML,
        len: 193,
        fnv1a64: 0x5e370f80030562eb,
    },
    LegacyBodyFingerprint {
        relative: PLAN_ADR_FORMAT,
        len: 2208,
        fnv1a64: 0x8b527e3ae5a8916c,
    },
    LegacyBodyFingerprint {
        relative: PLAN_SUBPLAN_BRIEF,
        len: 3586,
        fnv1a64: 0xc8d942a06e10775e,
    },
    LegacyBodyFingerprint {
        relative: PLAN_GOAL_BRIEF_FORMAT,
        len: 1818,
        fnv1a64: 0x2bf7a05a412c9df7,
    },
    LegacyBodyFingerprint {
        relative: PLAN_GOAL_BRIEF_FORMAT,
        len: 5918,
        fnv1a64: 0xdaf848443c7c9c74,
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
pub(super) fn legacy_marker_body_for_test(version: u32) -> String {
    let release = match version {
        14 => {
            return serde_json::to_string_pretty(&BundleMarker {
                managed_by: "codex1-managed".into(),
                version,
                files: files_to_strings(&pre_plan_retirement_bundle_files()),
            })
            .unwrap()
                + "\n"
        }
        13 => LEGACY_RELEASES[0],
        11 => LEGACY_RELEASES[1],
        8 => LEGACY_RELEASES[2],
        6 => LEGACY_RELEASES[3],
        5 => LEGACY_RELEASES[4],
        4 => LEGACY_RELEASES[5],
        3 => LEGACY_RELEASES[6],
        2 => LEGACY_RELEASES[7],
        1 => LEGACY_RELEASES[8],
        _ => panic!("unknown legacy release version: {version}"),
    };
    serde_json::to_string_pretty(&BundleMarker {
        managed_by: "codex1-managed".into(),
        version,
        files: files_to_strings(&legacy_bundle_files(release)),
    })
    .unwrap()
        + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn current_bundle_entries_have_no_duplicates() {
        let mut seen = HashSet::new();
        for entry in CURRENT_BUNDLE_ENTRIES {
            assert!(
                seen.insert(entry.relative),
                "duplicate managed setup file: {}",
                entry.relative
            );
        }
    }

    #[test]
    fn current_bundle_entries_are_fully_classified() {
        assert!(entries_by_role(BundleFileRole::ManagedSkill).count() > 0);
        assert!(entries_by_role(BundleFileRole::SupportingDoc).count() > 0);
        assert_eq!(entries_by_role(BundleFileRole::Guidance).count(), 1);
        assert_eq!(
            owned_file_entries().count() + entries_by_role(BundleFileRole::Guidance).count(),
            CURRENT_BUNDLE_ENTRIES.len()
        );
    }

    #[test]
    fn current_managed_files_have_expected_bodies() {
        for entry in CURRENT_BUNDLE_ENTRIES {
            assert!(
                expected_body(entry.relative).is_some(),
                "{}",
                entry.relative
            );
        }
    }

    #[test]
    fn expected_bodies_match_checked_in_bundle_files() {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        for entry in CURRENT_BUNDLE_ENTRIES {
            let checked_in = std::fs::read_to_string(repo.join(entry.relative)).unwrap();
            match entry.role {
                BundleFileRole::Guidance => {
                    assert!(
                        checked_in.contains(&guidance::managed_block()),
                        "{}",
                        entry.relative
                    );
                }
                BundleFileRole::ManagedSkill | BundleFileRole::SupportingDoc => {
                    assert_eq!(entry.expected_body(), checked_in, "{}", entry.relative);
                }
            }
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
    fn legacy_release_specs_match_known_compatibility_markers() {
        for version in [14, 13, 11, 8, 6, 5, 4, 3, 2, 1] {
            let marker: BundleMarker =
                serde_json::from_str(&legacy_marker_body_for_test(version)).unwrap();
            assert!(
                is_managed_bundle_marker(&marker),
                "legacy release {version}"
            );
        }
    }

    #[test]
    fn retired_legacy_files_are_not_current_bundle_files() {
        for relative in retired_legacy_files() {
            assert!(!is_current_bundle_file(relative), "{relative}");
        }
    }

    #[test]
    fn retired_execution_prompt_body_is_managed_body_proof() {
        assert!(is_managed_restore_body(
            LEGACY_PLAN_EXECUTION_PROMPT_FORMAT,
            legacy_execution_prompt_format_body()
        ));
    }

    #[test]
    fn legacy_fingerprints_cover_recent_managed_body_changes() {
        for (relative, len, fnv1a64) in [
            (CLARIFY_SKILL, 3919, 0xb75e2bb26bbfe162),
            (CREATE_PRD_SKILL, 3221, 0xe77c78c652529d1e),
            (CREATE_PRD_FORMAT, 1834, 0x0c53c184ad841164),
            (PLAN_SKILL, 12566, 0xf0bb154ce15a5f52),
            (PLAN_OPENAI_YAML, 193, 0x5e370f80030562eb),
            (PLAN_ADR_FORMAT, 2208, 0x8b527e3ae5a8916c),
            (PLAN_SUBPLAN_BRIEF, 3586, 0xc8d942a06e10775e),
            (PLAN_GOAL_BRIEF_FORMAT, 5918, 0xdaf848443c7c9c74),
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
