use std::collections::HashSet;

use serde::Serialize;

use crate::error::{Codex1Error, Result};
use crate::layout::ArtifactKind;

#[derive(Clone, Debug, Serialize)]
pub struct Template {
    pub kind: ArtifactKind,
    pub version: u32,
    pub name: &'static str,
    pub sections: &'static [Section],
}

#[derive(Clone, Debug, Serialize)]
pub struct Section {
    pub id: &'static str,
    pub heading: &'static str,
    pub required: bool,
    pub prompt: &'static str,
    pub repeatable: bool,
}

macro_rules! section {
    ($id:literal, $heading:literal, required, $prompt:literal) => {
        Section {
            id: $id,
            heading: $heading,
            required: true,
            prompt: $prompt,
            repeatable: false,
        }
    };
    ($id:literal, $heading:literal, optional, $prompt:literal) => {
        Section {
            id: $id,
            heading: $heading,
            required: false,
            prompt: $prompt,
            repeatable: false,
        }
    };
    ($id:literal, $heading:literal, required_list, $prompt:literal) => {
        Section {
            id: $id,
            heading: $heading,
            required: true,
            prompt: $prompt,
            repeatable: true,
        }
    };
    ($id:literal, $heading:literal, optional_list, $prompt:literal) => {
        Section {
            id: $id,
            heading: $heading,
            required: false,
            prompt: $prompt,
            repeatable: true,
        }
    };
}

const PRD: &[Section] = &[
    section!("title", "Title", required, "What is the mission title?"),
    section!(
        "original_request",
        "Original Request",
        required,
        "What did the user ask for?"
    ),
    section!(
        "interpreted_destination",
        "Interpreted Destination",
        required,
        "What destination should Codex aim for?"
    ),
    section!(
        "success_criteria",
        "Success Criteria",
        required_list,
        "What must be true?"
    ),
    section!(
        "non_goals",
        "Non-Goals",
        optional_list,
        "What is out of scope?"
    ),
    section!(
        "constraints",
        "Constraints",
        optional_list,
        "What constraints apply?"
    ),
    section!(
        "verified_context",
        "Verified Context",
        optional_list,
        "What context was verified?"
    ),
    section!(
        "assumptions",
        "Assumptions",
        optional_list,
        "What assumptions remain?"
    ),
    section!(
        "resolved_questions",
        "Resolved Questions",
        optional_list,
        "What questions were resolved?"
    ),
    section!(
        "proof_expectations",
        "Proof Expectations",
        required_list,
        "What proof is expected?"
    ),
    section!(
        "review_expectations",
        "Review Expectations",
        optional_list,
        "What review is expected?"
    ),
    section!("pr_intent", "PR Intent", required, "Should a PR be opened?"),
];

const PLAN: &[Section] = &[
    section!("title", "Title", required, "What is the plan title?"),
    section!(
        "mission_link",
        "Mission Link",
        required,
        "Which PRD does this plan serve?"
    ),
    section!(
        "strategy_thesis",
        "Strategy Thesis",
        required,
        "What is the strategy?"
    ),
    section!(
        "workstreams",
        "Workstreams",
        required_list,
        "What are the workstreams?"
    ),
    section!("phases", "Phases", required_list, "What are the phases?"),
    section!(
        "research_posture",
        "Research Posture",
        optional,
        "What research is needed?"
    ),
    section!("risk_map", "Risk Map", optional_list, "What risks matter?"),
    section!(
        "artifact_index",
        "Artifact Index",
        optional_list,
        "Which artifacts matter?"
    ),
    section!(
        "review_posture",
        "Review Posture",
        optional,
        "What review posture applies?"
    ),
    section!(
        "recommended_next_slices",
        "Recommended Next Slices",
        required_list,
        "What slices are next?"
    ),
];

const RESEARCH_PLAN: &[Section] = &[
    section!(
        "title",
        "Title",
        required,
        "What is the research plan title?"
    ),
    section!(
        "research_questions",
        "Research Questions",
        required_list,
        "What questions need research?"
    ),
    section!(
        "sources_to_inspect",
        "Sources To Inspect",
        required_list,
        "What sources should be inspected?"
    ),
    section!(
        "experiments_to_run",
        "Experiments To Run",
        optional_list,
        "What experiments should run?"
    ),
    section!(
        "expected_outputs",
        "Expected Outputs",
        required_list,
        "What outputs should research create?"
    ),
    section!(
        "stopping_criteria",
        "Stopping Criteria",
        required_list,
        "When is research enough?"
    ),
    section!(
        "plan_effect",
        "How Findings Affect The Plan",
        required,
        "How will findings affect planning?"
    ),
];

const RESEARCH: &[Section] = &[
    section!(
        "title",
        "Title",
        required,
        "What is the research record title?"
    ),
    section!(
        "question",
        "Question",
        required,
        "What question was researched?"
    ),
    section!(
        "sources_inspected",
        "Sources Inspected",
        required_list,
        "What sources were inspected?"
    ),
    section!(
        "facts_found",
        "Facts Found",
        required_list,
        "What facts were found?"
    ),
    section!(
        "experiments_run",
        "Experiments Run",
        optional_list,
        "What experiments ran?"
    ),
    section!(
        "uncertainties",
        "Uncertainties",
        optional_list,
        "What remains uncertain?"
    ),
    section!(
        "options_considered",
        "Options Considered",
        optional_list,
        "What options were considered?"
    ),
    section!(
        "recommendation",
        "Recommendation",
        required,
        "What is the recommendation?"
    ),
    section!(
        "affected_artifacts",
        "Affected Artifacts",
        optional_list,
        "What artifacts are affected?"
    ),
];

const SPEC: &[Section] = &[
    section!("title", "Title", required, "What is the spec title?"),
    section!(
        "responsibility",
        "Responsibility",
        required,
        "What responsibility does this spec cover?"
    ),
    section!(
        "prd_relevance",
        "PRD Relevance",
        required,
        "How does it serve the PRD?"
    ),
    section!("scope", "Scope", required_list, "What is in scope?"),
    section!(
        "non_goals",
        "Non-Goals",
        optional_list,
        "What is out of scope?"
    ),
    section!(
        "expected_behavior",
        "Expected Behavior",
        required_list,
        "What behavior is expected?"
    ),
    section!(
        "interfaces_contracts",
        "Interfaces And Contracts",
        optional_list,
        "What interfaces matter?"
    ),
    section!(
        "implementation_notes",
        "Implementation Notes",
        optional_list,
        "What implementation notes apply?"
    ),
    section!(
        "proof_expectations",
        "Proof Expectations",
        required_list,
        "What proof is expected?"
    ),
    section!("risks", "Risks", optional_list, "What risks matter?"),
    section!(
        "revision_notes",
        "Revision Notes",
        optional_list,
        "What changed in this spec?"
    ),
];

const SUBPLAN: &[Section] = &[
    section!("title", "Title", required, "What is the subplan title?"),
    section!("goal", "Goal", required, "What is the slice goal?"),
    section!(
        "linked_prd",
        "Linked PRD",
        required,
        "Which PRD does it serve?"
    ),
    section!(
        "linked_plan",
        "Linked Plan",
        required,
        "Which plan does it serve?"
    ),
    section!(
        "linked_specs",
        "Linked Specs",
        optional_list,
        "Which specs are linked?"
    ),
    section!("owner", "Owner", required, "Who owns this slice?"),
    section!("scope", "Scope", required_list, "What is in scope?"),
    section!(
        "steps",
        "Steps",
        required_list,
        "What steps should be taken?"
    ),
    section!(
        "dependencies",
        "Dependencies",
        optional_list,
        "What dependencies exist?"
    ),
    section!(
        "expected_proof",
        "Expected Proof",
        required_list,
        "What proof is expected?"
    ),
    section!(
        "exit_criteria",
        "Exit Criteria",
        required_list,
        "What exits the slice?"
    ),
    section!(
        "handoff_notes",
        "Handoff Notes",
        optional_list,
        "What handoff notes matter?"
    ),
];

const ADR: &[Section] = &[
    section!("title", "Title", required, "What is the ADR title?"),
    section!("status", "Status", required, "What is the ADR status?"),
    section!(
        "context",
        "Context",
        required,
        "What context led to this decision?"
    ),
    section!("decision", "Decision", required, "What was decided?"),
    section!(
        "options_considered",
        "Options Considered",
        required_list,
        "What options were considered?"
    ),
    section!(
        "tradeoffs",
        "Tradeoffs",
        required_list,
        "What tradeoffs exist?"
    ),
    section!(
        "consequences",
        "Consequences",
        required_list,
        "What follows from this decision?"
    ),
    section!(
        "artifact_links",
        "Links To PRD/Plan/Specs",
        optional_list,
        "What artifacts are linked?"
    ),
];

const REVIEW: &[Section] = &[
    section!("title", "Title", required, "What is the review title?"),
    section!(
        "target",
        "Target Artifact Or Code Area",
        required,
        "What is being reviewed?"
    ),
    section!(
        "reviewer_role",
        "Reviewer Role",
        required,
        "What role is the reviewer taking?"
    ),
    section!(
        "overall_assessment",
        "Overall Assessment",
        required,
        "What is the assessment?"
    ),
    section!(
        "confidence",
        "Confidence",
        required,
        "What is the confidence?"
    ),
    section!(
        "findings",
        "Findings",
        optional_list,
        "What findings were identified?"
    ),
    section!(
        "non_blocking_notes",
        "Non-Blocking Notes",
        optional_list,
        "What notes are non-blocking?"
    ),
    section!(
        "recommended_followup",
        "Recommended Follow-Up",
        optional_list,
        "What follow-up is recommended?"
    ),
];

const TRIAGE: &[Section] = &[
    section!("title", "Title", required, "What is the triage title?"),
    section!(
        "linked_review",
        "Linked Review",
        required,
        "Which review is linked?"
    ),
    section!(
        "accepted_findings",
        "Accepted Findings",
        optional_list,
        "What findings are accepted?"
    ),
    section!(
        "rejected_findings",
        "Rejected Findings",
        optional_list,
        "What findings are rejected?"
    ),
    section!(
        "deferred_findings",
        "Deferred Findings",
        optional_list,
        "What findings are deferred?"
    ),
    section!(
        "duplicate_stale_findings",
        "Duplicate Or Stale Findings",
        optional_list,
        "What is duplicate or stale?"
    ),
    section!(
        "rationale",
        "Rationale",
        required,
        "What is the triage rationale?"
    ),
    section!(
        "artifact_changes",
        "Artifact Changes To Make",
        optional_list,
        "What artifacts should change?"
    ),
];

const PROOF: &[Section] = &[
    section!("title", "Title", required, "What is the proof title?"),
    section!(
        "linked_subplan",
        "Linked Subplan",
        required,
        "Which subplan is linked?"
    ),
    section!(
        "linked_spec",
        "Linked Spec",
        optional,
        "Which spec is linked?"
    ),
    section!(
        "summary_of_changes",
        "Summary Of Changes",
        required,
        "What changed?"
    ),
    section!(
        "commands_run",
        "Commands Run",
        required_list,
        "What commands ran?"
    ),
    section!("tests_run", "Tests Run", optional_list, "What tests ran?"),
    section!(
        "manual_checks",
        "Manual Checks",
        optional_list,
        "What manual checks ran?"
    ),
    section!(
        "changed_areas",
        "Changed Areas",
        required_list,
        "What areas changed?"
    ),
    section!("failures", "Failures", optional_list, "What failed?"),
    section!(
        "accepted_risks",
        "Accepted Risks",
        optional_list,
        "What risks remain accepted?"
    ),
    section!(
        "evidence_links",
        "Evidence Links",
        optional_list,
        "What evidence links exist?"
    ),
];

const CLOSEOUT: &[Section] = &[
    section!("title", "Title", required, "What is the closeout title?"),
    section!(
        "prd_satisfaction_summary",
        "PRD Satisfaction Summary",
        required,
        "How was the PRD satisfied?"
    ),
    section!(
        "completed_subplans",
        "Completed Subplans",
        optional_list,
        "What subplans completed?"
    ),
    section!(
        "superseded_subplans",
        "Superseded Subplans",
        optional_list,
        "What was superseded?"
    ),
    section!(
        "paused_deferred_subplans",
        "Paused Or Deferred Subplans",
        optional_list,
        "What paused or deferred?"
    ),
    section!("proofs", "Proofs", optional_list, "What proofs exist?"),
    section!(
        "reviews_triage",
        "Reviews And Triage",
        optional_list,
        "What reviews and triage happened?"
    ),
    section!("adrs", "ADRs", optional_list, "What ADRs matter?"),
    section!(
        "remaining_risks",
        "Remaining Risks",
        optional_list,
        "What risks remain?"
    ),
    section!(
        "pr_readiness",
        "PR Readiness",
        required,
        "What is the PR readiness or intent?"
    ),
    section!(
        "final_notes",
        "Final Notes",
        optional,
        "What final notes matter?"
    ),
];

pub fn get(kind: ArtifactKind) -> Template {
    let sections = match kind {
        ArtifactKind::Prd => PRD,
        ArtifactKind::Plan => PLAN,
        ArtifactKind::ResearchPlan => RESEARCH_PLAN,
        ArtifactKind::Research => RESEARCH,
        ArtifactKind::Spec => SPEC,
        ArtifactKind::Subplan => SUBPLAN,
        ArtifactKind::Adr => ADR,
        ArtifactKind::Review => REVIEW,
        ArtifactKind::Triage => TRIAGE,
        ArtifactKind::Proof => PROOF,
        ArtifactKind::Closeout => CLOSEOUT,
    };
    Template {
        kind,
        version: 1,
        name: kind.title(),
        sections,
    }
}

pub fn all() -> Vec<Template> {
    ArtifactKind::ALL.into_iter().map(get).collect()
}

pub fn validate_registry() -> Result<()> {
    let mut seen = HashSet::new();
    for template in all() {
        if template.version != 1 {
            return Err(Codex1Error::Template(format!(
                "{} has unsupported version {}",
                template.kind, template.version
            )));
        }
        if !seen.insert(template.kind) {
            return Err(Codex1Error::Template(format!(
                "duplicate template for {}",
                template.kind
            )));
        }
        let mut section_ids = HashSet::new();
        for section in template.sections {
            if !section_ids.insert(section.id) {
                return Err(Codex1Error::Template(format!(
                    "duplicate section {} in {}",
                    section.id, template.kind
                )));
            }
        }
    }
    Ok(())
}
