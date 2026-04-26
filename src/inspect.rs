use std::fs;
use std::io::ErrorKind;
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
        inspect_required_dir(layout, &dir, "MISSING_STANDARD_DIRECTORY", &mut warnings)?;
    }

    for state in SubplanState::ALL {
        let dir = layout.subplans_dir().join(state.as_str());
        inspect_required_dir_with_detail(
            &dir,
            "MISSING_SUBPLAN_DIRECTORY",
            format!("SUBPLANS lifecycle folder index {}", state_index(state)),
            layout,
            &mut warnings,
        )?;
    }

    counts.prd = exists(
        layout,
        layout.singleton_path(ArtifactKind::Prd)?.as_path(),
        &mut warnings,
    )?;
    counts.plan = exists(
        layout,
        layout.singleton_path(ArtifactKind::Plan)?.as_path(),
        &mut warnings,
    )?;
    counts.research_plan = exists(
        layout,
        layout.singleton_path(ArtifactKind::ResearchPlan)?.as_path(),
        &mut warnings,
    )?;
    counts.closeout = exists(
        layout,
        layout.singleton_path(ArtifactKind::Closeout)?.as_path(),
        &mut warnings,
    )?;
    counts.research = count_md(layout, &layout.research_dir(), &mut warnings)?;
    counts.specs = count_md(layout, &layout.specs_dir(), &mut warnings)?;
    counts.subplans = count_md_recursive(layout, &layout.subplans_dir(), &mut warnings)?;
    counts.adrs = count_md(layout, &layout.adrs_dir(), &mut warnings)?;
    counts.reviews = count_md(layout, &layout.reviews_dir(), &mut warnings)?;
    counts.triage = count_md(layout, &layout.triage_dir(), &mut warnings)?;
    counts.proofs = count_md(layout, &layout.proofs_dir(), &mut warnings)?;
    counts.optional_receipts = count_jsonl(layout, &layout.receipts_dir(), &mut warnings)?;

    for file in singleton_files(layout)? {
        if regular_file_exists(layout, &file, &mut warnings)? {
            validate_frontmatter(layout, &file, &mut warnings)?;
        }
    }
    validate_collection_frontmatter(layout, &layout.research_dir(), false, &mut warnings)?;
    validate_collection_frontmatter(layout, &layout.specs_dir(), false, &mut warnings)?;
    validate_collection_frontmatter(layout, &layout.subplans_dir(), true, &mut warnings)?;
    validate_collection_frontmatter(layout, &layout.adrs_dir(), false, &mut warnings)?;
    validate_collection_frontmatter(layout, &layout.reviews_dir(), false, &mut warnings)?;
    validate_collection_frontmatter(layout, &layout.triage_dir(), false, &mut warnings)?;
    validate_collection_frontmatter(layout, &layout.proofs_dir(), false, &mut warnings)?;

    Ok(Inventory {
        mission_id: layout.mission_id.clone(),
        mission_dir: layout.mission_dir.display().to_string(),
        artifacts: counts,
        mechanical_warnings: warnings,
    })
}

fn validate_frontmatter(
    layout: &MissionLayout,
    file: &Path,
    warnings: &mut Vec<MechanicalWarning>,
) -> Result<()> {
    let text = fs::read_to_string(file).io_context(format!("failed to read {}", file.display()))?;
    if !has_valid_frontmatter(&text) {
        warnings.push(MechanicalWarning {
            code: "MALFORMED_FRONTMATTER",
            detail: display_inside(layout, file),
        });
    }
    Ok(())
}

fn has_valid_frontmatter(text: &str) -> bool {
    text.starts_with("---\n") && text.lines().skip(1).any(|line| line == "---")
}

fn singleton_files(layout: &MissionLayout) -> Result<Vec<std::path::PathBuf>> {
    Ok(vec![
        layout.singleton_path(ArtifactKind::Prd)?,
        layout.singleton_path(ArtifactKind::Plan)?,
        layout.singleton_path(ArtifactKind::ResearchPlan)?,
        layout.singleton_path(ArtifactKind::Closeout)?,
    ])
}

fn inspect_required_dir(
    layout: &MissionLayout,
    dir: &Path,
    missing_code: &'static str,
    warnings: &mut Vec<MechanicalWarning>,
) -> Result<()> {
    inspect_required_dir_with_detail(
        dir,
        missing_code,
        display_inside(layout, dir),
        layout,
        warnings,
    )
}

fn inspect_required_dir_with_detail(
    dir: &Path,
    missing_code: &'static str,
    missing_detail: String,
    layout: &MissionLayout,
    warnings: &mut Vec<MechanicalWarning>,
) -> Result<()> {
    match fs::symlink_metadata(dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            warnings.push(MechanicalWarning {
                code: "SYMLINKED_PATH",
                detail: display_inside(layout, dir),
            });
        }
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) | Err(_) => warnings.push(MechanicalWarning {
            code: missing_code,
            detail: missing_detail,
        }),
    }
    Ok(())
}

fn exists(
    layout: &MissionLayout,
    path: &Path,
    warnings: &mut Vec<MechanicalWarning>,
) -> Result<usize> {
    Ok(usize::from(regular_file_exists(layout, path, warnings)?))
}

fn regular_file_exists(
    layout: &MissionLayout,
    path: &Path,
    warnings: &mut Vec<MechanicalWarning>,
) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            warnings.push(MechanicalWarning {
                code: "SYMLINKED_PATH",
                detail: display_inside(layout, path),
            });
            Ok(false)
        }
        Ok(metadata) => Ok(metadata.is_file()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(crate::error::Codex1Error::Io {
            context: format!("failed to inspect {}", path.display()),
            source: error,
        }),
    }
}

fn count_md(
    layout: &MissionLayout,
    dir: &Path,
    warnings: &mut Vec<MechanicalWarning>,
) -> Result<usize> {
    count_matching(layout, dir, false, "md", warnings)
}

fn count_md_recursive(
    layout: &MissionLayout,
    dir: &Path,
    warnings: &mut Vec<MechanicalWarning>,
) -> Result<usize> {
    count_matching(layout, dir, true, "md", warnings)
}

fn count_jsonl(
    layout: &MissionLayout,
    dir: &Path,
    warnings: &mut Vec<MechanicalWarning>,
) -> Result<usize> {
    count_matching(layout, dir, false, "jsonl", warnings)
}

fn count_matching(
    layout: &MissionLayout,
    dir: &Path,
    recursive: bool,
    extension: &str,
    warnings: &mut Vec<MechanicalWarning>,
) -> Result<usize> {
    match fs::symlink_metadata(dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            warnings.push(MechanicalWarning {
                code: "SYMLINKED_PATH",
                detail: display_inside(layout, dir),
            });
            return Ok(0);
        }
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Ok(0),
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(crate::error::Codex1Error::Io {
                context: format!("failed to inspect {}", dir.display()),
                source: error,
            })
        }
    }
    let mut count = 0;
    for entry in fs::read_dir(dir).io_context(format!("failed to read {}", dir.display()))? {
        let entry = entry.io_context(format!("failed to read entry in {}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .io_context(format!("failed to inspect entry in {}", dir.display()))?;
        if file_type.is_symlink() {
            warnings.push(MechanicalWarning {
                code: "SYMLINKED_PATH",
                detail: display_inside(layout, &path),
            });
            continue;
        }
        if recursive && file_type.is_dir() {
            count += count_matching(layout, &path, true, extension, warnings)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            count += 1;
        }
    }
    Ok(count)
}

fn validate_collection_frontmatter(
    layout: &MissionLayout,
    dir: &Path,
    recursive: bool,
    warnings: &mut Vec<MechanicalWarning>,
) -> Result<()> {
    match fs::symlink_metadata(dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => return Ok(()),
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(crate::error::Codex1Error::Io {
                context: format!("failed to inspect {}", dir.display()),
                source: error,
            })
        }
    }

    for entry in fs::read_dir(dir).io_context(format!("failed to read {}", dir.display()))? {
        let entry = entry.io_context(format!("failed to read entry in {}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .io_context(format!("failed to inspect entry in {}", dir.display()))?;
        if file_type.is_symlink() {
            continue;
        }
        if recursive && file_type.is_dir() {
            validate_collection_frontmatter(layout, &path, true, warnings)?;
        } else if file_type.is_file()
            && path.extension().and_then(|value| value.to_str()) == Some("md")
        {
            validate_frontmatter(layout, &path, warnings)?;
        }
    }
    Ok(())
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
