use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};
use crate::fingerprint::Fingerprint;
use crate::runtime::ReplanBoundary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    MissionState,
    OutcomeLock,
    ProgramBlueprint,
    WorkstreamSpec,
}

impl ArtifactKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissionState => "mission-state",
            Self::OutcomeLock => "outcome-lock",
            Self::ProgramBlueprint => "program-blueprint",
            Self::WorkstreamSpec => "workstream-spec",
        }
    }
}

pub trait TypedArtifactFrontmatter:
    Clone + PartialEq + Eq + Serialize + DeserializeOwned + std::fmt::Debug
{
    const KIND: ArtifactKind;

    fn artifact_kind(&self) -> ArtifactKind;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibleArtifactTextKind {
    MissionReadme,
    ReviewLedger,
    ReplanLog,
}

impl VisibleArtifactTextKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissionReadme => "mission_readme",
            Self::ReviewLedger => "review_ledger",
            Self::ReplanLog => "replan_log",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleArtifactSectionRequirement {
    pub heading: String,
    #[serde(default)]
    pub required_phrases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleArtifactTextRequirement {
    pub kind: VisibleArtifactTextKind,
    #[serde(default)]
    pub required_headings: Vec<String>,
    #[serde(default)]
    pub required_phrases: Vec<String>,
    #[serde(default)]
    pub section_requirements: Vec<VisibleArtifactSectionRequirement>,
}

#[must_use]
pub fn visible_artifact_text_requirement(
    kind: VisibleArtifactTextKind,
) -> VisibleArtifactTextRequirement {
    match kind {
        VisibleArtifactTextKind::MissionReadme => VisibleArtifactTextRequirement {
            kind,
            required_headings: vec![
                "Snapshot".to_string(),
                "Start Here".to_string(),
                "Objective Summary".to_string(),
                "Active Frontier".to_string(),
                "Current Risks Or Blockers".to_string(),
                "Canonical Artifacts".to_string(),
            ],
            required_phrases: vec![
                "Mission id:".to_string(),
                "Current phase:".to_string(),
                "Current verdict:".to_string(),
                "Next recommended action:".to_string(),
                "Current blocker:".to_string(),
            ],
            section_requirements: vec![
                VisibleArtifactSectionRequirement {
                    heading: "Start Here".to_string(),
                    required_phrases: vec![
                        "OUTCOME-LOCK.md".to_string(),
                        "PROGRAM-BLUEPRINT.md".to_string(),
                    ],
                },
                VisibleArtifactSectionRequirement {
                    heading: "Active Frontier".to_string(),
                    required_phrases: vec![
                        "Selected target:".to_string(),
                        "Why it is next:".to_string(),
                        "Expected proof, review, or package gate:".to_string(),
                    ],
                },
                VisibleArtifactSectionRequirement {
                    heading: "Canonical Artifacts".to_string(),
                    required_phrases: vec![
                        "MISSION-STATE.md".to_string(),
                        "OUTCOME-LOCK.md".to_string(),
                        "PROGRAM-BLUEPRINT.md".to_string(),
                    ],
                },
            ],
        },
        VisibleArtifactTextKind::ReviewLedger => VisibleArtifactTextRequirement {
            kind,
            required_headings: vec![
                "Open Blocking Findings".to_string(),
                "Non-Blocking Findings".to_string(),
                "Review Events".to_string(),
                "Dispositions".to_string(),
                "Mission-Close Review".to_string(),
            ],
            required_phrases: vec!["Mission id:".to_string()],
            section_requirements: vec![VisibleArtifactSectionRequirement {
                heading: "Mission-Close Review".to_string(),
                required_phrases: vec![
                    "Bundle id:".to_string(),
                    "Source package id:".to_string(),
                    "Governing refs:".to_string(),
                    "Verdict:".to_string(),
                ],
            }],
        },
        VisibleArtifactTextKind::ReplanLog => VisibleArtifactTextRequirement {
            kind,
            required_headings: vec!["Notes".to_string()],
            required_phrases: vec![
                "Mission id:".to_string(),
                "| Replan id |".to_string(),
                "| Reopened layer |".to_string(),
                "| Trigger |".to_string(),
                "| Cause ref |".to_string(),
                "| Preserved work |".to_string(),
                "| Invalidated work |".to_string(),
                "| Artifact updates |".to_string(),
            ],
            section_requirements: vec![VisibleArtifactSectionRequirement {
                heading: "Notes".to_string(),
                required_phrases: vec![
                    "Preserve valid work".to_string(),
                    "invalidated".to_string(),
                ],
            }],
        },
    }
}

fn normalize_markdown_heading(heading: &str) -> String {
    heading
        .trim()
        .trim_start_matches('#')
        .trim()
        .to_ascii_lowercase()
}

fn markdown_sections(body: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current_heading = String::new();
    let mut current_lines: Vec<String> = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            if !current_heading.is_empty() {
                sections.push((current_heading.clone(), current_lines.join("\n")));
            }
            current_heading = normalize_markdown_heading(trimmed);
            current_lines.clear();
            continue;
        }

        if !current_heading.is_empty() {
            current_lines.push(line.to_string());
        }
    }

    if !current_heading.is_empty() {
        sections.push((current_heading, current_lines.join("\n")));
    }

    sections
}

#[must_use]
pub fn validate_visible_artifact_text(
    kind: VisibleArtifactTextKind,
    contents: &str,
) -> Vec<String> {
    let requirement = visible_artifact_text_requirement(kind);
    let mut findings = Vec::new();
    if contents.trim().is_empty() {
        findings.push("artifact is empty".to_string());
        return findings;
    }

    let sections = markdown_sections(contents);
    for heading in &requirement.required_headings {
        let normalized = normalize_markdown_heading(heading);
        if !sections
            .iter()
            .any(|(candidate, _)| candidate == &normalized)
        {
            findings.push(format!("artifact is missing required section `{heading}`"));
        }
    }

    for phrase in &requirement.required_phrases {
        if !contents.contains(phrase) {
            findings.push(format!("artifact is missing required phrase `{phrase}`"));
        }
    }

    for section_requirement in &requirement.section_requirements {
        let normalized = normalize_markdown_heading(&section_requirement.heading);
        if let Some((_, section_body)) = sections
            .iter()
            .find(|(candidate, _)| candidate == &normalized)
        {
            if section_body.trim().is_empty() {
                findings.push(format!(
                    "artifact section `{}` must not be empty",
                    section_requirement.heading
                ));
            }
            for phrase in &section_requirement.required_phrases {
                if !section_body.contains(phrase) {
                    findings.push(format!(
                        "artifact section `{}` is missing required phrase `{}`",
                        section_requirement.heading, phrase
                    ));
                }
            }
        }
    }

    findings
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactDocument<F> {
    pub frontmatter: F,
    pub body: String,
}

const fn default_risk_floor() -> u8 {
    1
}

impl<F> ArtifactDocument<F>
where
    F: TypedArtifactFrontmatter,
{
    pub fn parse(input: &str) -> Result<Self> {
        let (frontmatter_yaml, body) = split_frontmatter(input)?;
        let frontmatter: F = serde_yaml::from_str(&frontmatter_yaml)?;

        if frontmatter.artifact_kind() != F::KIND {
            return Err(CoreError::ArtifactKindMismatch {
                expected: F::KIND.as_str(),
                found: frontmatter.artifact_kind().as_str().to_owned(),
            });
        }

        Ok(Self { frontmatter, body })
    }

    pub fn render(&self) -> Result<String> {
        let yaml = serde_yaml::to_string(&self.frontmatter)?;
        let body = self.body.trim_end_matches('\n');

        if body.is_empty() {
            Ok(format!("---\n{}---\n", yaml))
        } else {
            Ok(format!("---\n{}---\n{}\n", yaml, body))
        }
    }

    pub fn fingerprint(&self) -> Result<Fingerprint> {
        let rendered = self.render()?;
        Ok(Fingerprint::from_bytes(rendered.as_bytes()))
    }
}

fn split_frontmatter(input: &str) -> Result<(String, String)> {
    let normalized = input.replace("\r\n", "\n");
    let Some(stripped) = normalized.strip_prefix("---\n") else {
        return Err(CoreError::MissingFrontmatter);
    };

    let Some((frontmatter, body)) = stripped.split_once("\n---\n") else {
        return Err(CoreError::InvalidFrontmatterDelimiter);
    };

    Ok((frontmatter.to_owned(), body.to_owned()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClarifyStatus {
    Clarifying,
    WaitingUser,
    Ratified,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockStatus {
    Draft,
    Locked,
    Reopened,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockPosture {
    Unconstrained,
    Constrained,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlueprintStatus {
    Draft,
    Approved,
    Reopened,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProblemSize {
    S,
    M,
    L,
    XL,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionBlockingness {
    Critical,
    Major,
    Minor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionAffect {
    ArchitectureBoundary,
    MigrationRollout,
    RollbackViability,
    ProofDesign,
    ReviewContract,
    ExecutionSequencing,
    BlastRadius,
    ProtectedSurfaceRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Open,
    Researched,
    Selected,
    ProofGatedSpike,
    NeedsUser,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofSpikeFailureRoute {
    ReplanRequired,
    NeedsUser,
    Descoped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofMatrixRow {
    pub claim_ref: String,
    pub statement: String,
    #[serde(default)]
    pub required_evidence: Vec<String>,
    #[serde(default)]
    pub review_lenses: Vec<String>,
    #[serde(default)]
    pub governing_contract_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionObligation {
    pub obligation_id: String,
    pub question: String,
    pub why_it_matters: String,
    #[serde(default)]
    pub affects: Vec<DecisionAffect>,
    #[serde(default)]
    pub governing_contract_refs: Vec<String>,
    #[serde(default)]
    pub review_contract_refs: Vec<String>,
    #[serde(default)]
    pub mission_close_claim_refs: Vec<String>,
    pub blockingness: DecisionBlockingness,
    pub candidate_route_count: u32,
    #[serde(default)]
    pub required_evidence: Vec<String>,
    pub status: DecisionStatus,
    #[serde(default)]
    pub resolution_rationale: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub proof_spike_scope: Option<String>,
    #[serde(default)]
    pub proof_spike_success_criteria: Vec<String>,
    #[serde(default)]
    pub proof_spike_failure_criteria: Vec<String>,
    #[serde(default)]
    pub proof_spike_discharge_artifacts: Vec<String>,
    #[serde(default)]
    pub proof_spike_failure_route: Option<ProofSpikeFailureRoute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecArtifactStatus {
    Draft,
    Active,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PacketizationStatus {
    Runnable,
    NearFrontier,
    ProofGatedSpike,
    ProvisionalBacklog,
    DeferredTruthMotion,
    Descoped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecExecutionStatus {
    NotStarted,
    Packaged,
    Executing,
    Blocked,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerMode {
    Solo,
    Delegated,
    Wave,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionStateFrontmatter {
    pub artifact: ArtifactKind,
    pub mission_id: String,
    pub root_mission_id: String,
    pub parent_mission_id: Option<String>,
    pub version: u32,
    pub clarify_status: ClarifyStatus,
    pub slug: String,
    pub current_lock_revision: Option<u64>,
    pub reopened_from_lock_revision: Option<u64>,
}

impl TypedArtifactFrontmatter for MissionStateFrontmatter {
    const KIND: ArtifactKind = ArtifactKind::MissionState;

    fn artifact_kind(&self) -> ArtifactKind {
        self.artifact
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeLockFrontmatter {
    pub artifact: ArtifactKind,
    pub mission_id: String,
    pub root_mission_id: String,
    pub parent_mission_id: Option<String>,
    pub version: u32,
    pub lock_revision: u64,
    pub status: LockStatus,
    pub lock_posture: LockPosture,
    pub slug: String,
}

impl TypedArtifactFrontmatter for OutcomeLockFrontmatter {
    const KIND: ArtifactKind = ArtifactKind::OutcomeLock;

    fn artifact_kind(&self) -> ArtifactKind {
        self.artifact
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramBlueprintFrontmatter {
    pub artifact: ArtifactKind,
    pub mission_id: String,
    pub version: u32,
    pub lock_revision: u64,
    pub blueprint_revision: u64,
    pub plan_level: u8,
    #[serde(default = "default_risk_floor")]
    pub risk_floor: u8,
    pub problem_size: Option<ProblemSize>,
    pub status: BlueprintStatus,
    #[serde(default)]
    pub proof_matrix: Vec<ProofMatrixRow>,
    #[serde(default)]
    pub decision_obligations: Vec<DecisionObligation>,
    #[serde(default)]
    pub selected_target_ref: Option<String>,
}

impl TypedArtifactFrontmatter for ProgramBlueprintFrontmatter {
    const KIND: ArtifactKind = ArtifactKind::ProgramBlueprint;

    fn artifact_kind(&self) -> ArtifactKind {
        self.artifact
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkstreamSpecFrontmatter {
    pub artifact: ArtifactKind,
    pub mission_id: String,
    pub spec_id: String,
    pub version: u32,
    pub spec_revision: u64,
    pub artifact_status: SpecArtifactStatus,
    pub packetization_status: PacketizationStatus,
    pub execution_status: SpecExecutionStatus,
    pub owner_mode: OwnerMode,
    pub blueprint_revision: u64,
    pub blueprint_fingerprint: Option<Fingerprint>,
    pub spec_fingerprint: Option<Fingerprint>,
    #[serde(default)]
    pub replan_boundary: Option<ReplanBoundary>,
}

impl TypedArtifactFrontmatter for WorkstreamSpecFrontmatter {
    const KIND: ArtifactKind = ArtifactKind::WorkstreamSpec;

    fn artifact_kind(&self) -> ArtifactKind {
        self.artifact
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ArtifactDocument, ArtifactKind, ClarifyStatus, MissionStateFrontmatter,
        TypedArtifactFrontmatter, VisibleArtifactTextKind, validate_visible_artifact_text,
    };

    #[test]
    fn parses_and_renders_mission_state_document() {
        let source = r#"---
artifact: mission-state
mission_id: mission-1
root_mission_id: mission-1
parent_mission_id: null
version: 1
clarify_status: clarifying
slug: fix-auth-flow
current_lock_revision: null
reopened_from_lock_revision: null
---
# Mission

Body text.
"#;

        let document = ArtifactDocument::<MissionStateFrontmatter>::parse(source)
            .expect("document should parse");

        assert_eq!(
            document.frontmatter.artifact_kind(),
            ArtifactKind::MissionState
        );
        assert_eq!(
            document.frontmatter.clarify_status,
            ClarifyStatus::Clarifying
        );
        assert!(document.body.contains("Body text"));

        let rendered = document.render().expect("document should render");
        let reparsed = ArtifactDocument::<MissionStateFrontmatter>::parse(&rendered)
            .expect("rendered document should parse");

        assert_eq!(document, reparsed);
        assert!(document.fingerprint().is_ok());
    }

    #[test]
    fn readme_template_satisfies_registered_contract() {
        let contents = include_str!("../../../templates/mission/README.md");
        let findings =
            validate_visible_artifact_text(VisibleArtifactTextKind::MissionReadme, contents);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn review_ledger_template_satisfies_registered_contract() {
        let contents = include_str!("../../../templates/mission/REVIEW-LEDGER.md");
        let findings =
            validate_visible_artifact_text(VisibleArtifactTextKind::ReviewLedger, contents);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn replan_log_template_satisfies_registered_contract() {
        let contents = include_str!("../../../templates/mission/REPLAN-LOG.md");
        let findings = validate_visible_artifact_text(VisibleArtifactTextKind::ReplanLog, contents);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }
}
