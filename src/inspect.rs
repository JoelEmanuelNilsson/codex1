use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::error::{IoContext, Result};
use crate::layout::{ArtifactKind, MissionLayout, SubplanState};

#[derive(Debug, Serialize)]
pub struct Inventory {
    pub mission_id: String,
    pub mission_dir: String,
    pub artifacts: ArtifactCounts,
    pub mechanical_warnings: Vec<MechanicalWarning>,
}

#[derive(Debug, Default, Serialize)]
pub struct ArtifactCounts {
    pub prd: usize,
    pub plan: usize,
    pub research_plan: usize,
    pub research: usize,
    pub specs: usize,
    pub subplans: usize,
    pub adrs: usize,
    pub reviews: usize,
    pub triage: usize,
    pub proofs: usize,
    pub closeout: usize,
    pub optional_receipts: usize,
}

#[derive(Debug, Serialize)]
pub struct MechanicalWarning {
    pub code: &'static str,
    pub detail: String,
}

pub fn inspect(layout: &MissionLayout) -> Result<Inventory> {
    let mut warnings = Vec::new();
    let mut counts = ArtifactCounts::default();

    let required_dirs = [
        layout.meta_dir(),
        layout.research_dir(),
        layout.specs_dir(),
        layout.subplans_dir(),
        layout.adrs_dir(),
        layout.reviews_dir(),
        layout.triage_dir(),
        layout.proofs_dir(),
    ];
    for dir in required_dirs {
        if !dir.is_dir() {
            warnings.push(MechanicalWarning {
                code: "MISSING_STANDARD_DIRECTORY",
                detail: display_inside(layout, &dir),
            });
        }
    }

    for state in SubplanState::ALL {
        let dir = layout.subplans_dir().join(state.as_str());
        if !dir.is_dir() {
            warnings.push(MechanicalWarning {
                code: "MISSING_SUBPLAN_DIRECTORY",
                detail: format!("SUBPLANS lifecycle folder index {}", state_index(state)),
            });
        }
    }

    counts.prd = exists(layout.singleton_path(ArtifactKind::Prd)?.as_path());
    counts.plan = exists(layout.singleton_path(ArtifactKind::Plan)?.as_path());
    counts.research_plan = exists(layout.singleton_path(ArtifactKind::ResearchPlan)?.as_path());
    counts.closeout = exists(layout.singleton_path(ArtifactKind::Closeout)?.as_path());
    counts.research = count_md(&layout.research_dir())?;
    counts.specs = count_md(&layout.specs_dir())?;
    counts.subplans = count_md_recursive(&layout.subplans_dir())?;
    counts.adrs = count_md(&layout.adrs_dir())?;
    counts.reviews = count_md(&layout.reviews_dir())?;
    counts.triage = count_md(&layout.triage_dir())?;
    counts.proofs = count_md(&layout.proofs_dir())?;
    counts.optional_receipts = count_jsonl(&layout.receipts_dir())?;

    for file in singleton_files(layout)? {
        if file.exists() {
            let text = fs::read_to_string(&file)
                .io_context(format!("failed to read {}", file.display()))?;
            if !text.starts_with("---\n") {
                warnings.push(MechanicalWarning {
                    code: "MALFORMED_FRONTMATTER",
                    detail: display_inside(layout, &file),
                });
            }
        }
    }

    Ok(Inventory {
        mission_id: layout.mission_id.clone(),
        mission_dir: layout.mission_dir.display().to_string(),
        artifacts: counts,
        mechanical_warnings: warnings,
    })
}

fn singleton_files(layout: &MissionLayout) -> Result<Vec<std::path::PathBuf>> {
    Ok(vec![
        layout.singleton_path(ArtifactKind::Prd)?,
        layout.singleton_path(ArtifactKind::Plan)?,
        layout.singleton_path(ArtifactKind::ResearchPlan)?,
        layout.singleton_path(ArtifactKind::Closeout)?,
    ])
}

fn exists(path: &Path) -> usize {
    usize::from(path.is_file())
}

fn count_md(dir: &Path) -> Result<usize> {
    count_matching(dir, false, "md")
}

fn count_md_recursive(dir: &Path) -> Result<usize> {
    count_matching(dir, true, "md")
}

fn count_jsonl(dir: &Path) -> Result<usize> {
    count_matching(dir, false, "jsonl")
}

fn count_matching(dir: &Path, recursive: bool, extension: &str) -> Result<usize> {
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in fs::read_dir(dir).io_context(format!("failed to read {}", dir.display()))? {
        let entry = entry.io_context(format!("failed to read entry in {}", dir.display()))?;
        let path = entry.path();
        if recursive && path.is_dir() {
            count += count_matching(&path, true, extension)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            count += 1;
        }
    }
    Ok(count)
}

fn display_inside(layout: &MissionLayout, path: &Path) -> String {
    path.strip_prefix(&layout.mission_dir)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn state_index(state: SubplanState) -> usize {
    match state {
        SubplanState::Ready => 0,
        SubplanState::Active => 1,
        SubplanState::Done => 2,
        SubplanState::Paused => 3,
        SubplanState::Superseded => 4,
    }
}
