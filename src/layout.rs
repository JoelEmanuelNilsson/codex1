use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::Serialize;

use crate::error::{Codex1Error, IoContext, Result};
use crate::paths::{create_dir_all_contained, safe_join, validate_mission_id};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    Prd,
    Plan,
    ResearchPlan,
    Research,
    Spec,
    Subplan,
    Adr,
    Review,
    Triage,
    Proof,
    Closeout,
}

impl ArtifactKind {
    pub const ALL: [Self; 11] = [
        Self::Prd,
        Self::Plan,
        Self::ResearchPlan,
        Self::Research,
        Self::Spec,
        Self::Subplan,
        Self::Adr,
        Self::Review,
        Self::Triage,
        Self::Proof,
        Self::Closeout,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prd => "prd",
            Self::Plan => "plan",
            Self::ResearchPlan => "research-plan",
            Self::Research => "research",
            Self::Spec => "spec",
            Self::Subplan => "subplan",
            Self::Adr => "adr",
            Self::Review => "review",
            Self::Triage => "triage",
            Self::Proof => "proof",
            Self::Closeout => "closeout",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Prd => "PRD",
            Self::Plan => "Plan",
            Self::ResearchPlan => "Research Plan",
            Self::Research => "Research Record",
            Self::Spec => "Spec",
            Self::Subplan => "Subplan",
            Self::Adr => "ADR",
            Self::Review => "Review",
            Self::Triage => "Triage",
            Self::Proof => "Proof",
            Self::Closeout => "Closeout",
        }
    }

    pub fn is_singleton(self) -> bool {
        matches!(
            self,
            Self::Prd | Self::Plan | Self::ResearchPlan | Self::Closeout
        )
    }
}

impl fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ArtifactKind {
    type Err = Codex1Error;

    fn from_str(s: &str) -> Result<Self> {
        ArtifactKind::ALL
            .into_iter()
            .find(|kind| kind.as_str() == s)
            .ok_or_else(|| Codex1Error::Argument(format!("unknown artifact kind: {s}")))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SubplanState {
    Ready,
    Active,
    Done,
    Paused,
    Superseded,
}

impl SubplanState {
    pub const ALL: [Self; 5] = [
        Self::Ready,
        Self::Active,
        Self::Done,
        Self::Paused,
        Self::Superseded,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Active => "active",
            Self::Done => "done",
            Self::Paused => "paused",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Clone, Debug)]
pub struct MissionLayout {
    pub repo_root: PathBuf,
    pub mission_id: String,
    pub mission_dir: PathBuf,
}

impl MissionLayout {
    pub fn new(repo_root: PathBuf, mission_id: String) -> Result<Self> {
        validate_mission_id(&mission_id)?;
        let missions_dir = repo_root.join(".codex1").join("missions");
        let mission_dir = missions_dir.join(&mission_id);
        validate_existing_mission_path(&repo_root, &mission_id)?;
        Ok(Self {
            repo_root,
            mission_id,
            mission_dir,
        })
    }

    pub fn from_cwd(repo_root: PathBuf, cwd: &Path) -> Option<Self> {
        let missions_dir = repo_root.join(".codex1").join("missions");
        let cwd = fs::canonicalize(cwd).ok()?;
        let missions_dir = fs::canonicalize(missions_dir).ok()?;
        let rel = cwd.strip_prefix(&missions_dir).ok()?;
        let id = rel
            .components()
            .next()?
            .as_os_str()
            .to_string_lossy()
            .to_string();
        Self::new(repo_root, id).ok()
    }

    pub fn create_dirs(&self) -> Result<()> {
        create_dir_all_contained(
            &self.repo_root,
            Path::new(".codex1").join("missions").join(&self.mission_id),
        )?;
        for relative in self.standard_dir_relatives() {
            create_dir_all_contained(&self.mission_dir, relative)?;
        }
        Ok(())
    }

    fn standard_dir_relatives(&self) -> Vec<PathBuf> {
        let mut dirs = vec![
            PathBuf::from(".codex1"),
            PathBuf::from("RESEARCH"),
            PathBuf::from("SPECS"),
            PathBuf::from("ADRS"),
            PathBuf::from("REVIEWS"),
            PathBuf::from("TRIAGE"),
            PathBuf::from("PROOFS"),
            PathBuf::from(".codex1").join("receipts"),
        ];
        for state in SubplanState::ALL {
            dirs.push(PathBuf::from("SUBPLANS").join(state.as_str()));
        }
        dirs
    }

    pub fn meta_dir(&self) -> PathBuf {
        self.mission_dir.join(".codex1")
    }

    pub fn loop_file(&self) -> PathBuf {
        self.meta_dir().join("LOOP.json")
    }

    pub fn event_log(&self) -> PathBuf {
        self.meta_dir().join("events.jsonl")
    }

    pub fn receipts_dir(&self) -> PathBuf {
        self.meta_dir().join("receipts")
    }

    pub fn research_dir(&self) -> PathBuf {
        self.mission_dir.join("RESEARCH")
    }

    pub fn specs_dir(&self) -> PathBuf {
        self.mission_dir.join("SPECS")
    }

    pub fn subplans_dir(&self) -> PathBuf {
        self.mission_dir.join("SUBPLANS")
    }

    pub fn adrs_dir(&self) -> PathBuf {
        self.mission_dir.join("ADRS")
    }

    pub fn reviews_dir(&self) -> PathBuf {
        self.mission_dir.join("REVIEWS")
    }

    pub fn triage_dir(&self) -> PathBuf {
        self.mission_dir.join("TRIAGE")
    }

    pub fn proofs_dir(&self) -> PathBuf {
        self.mission_dir.join("PROOFS")
    }

    pub fn singleton_path(&self, kind: ArtifactKind) -> Result<PathBuf> {
        match kind {
            ArtifactKind::Prd => safe_join(&self.mission_dir, "PRD.md"),
            ArtifactKind::Plan => safe_join(&self.mission_dir, "PLAN.md"),
            ArtifactKind::ResearchPlan => safe_join(&self.mission_dir, "RESEARCH_PLAN.md"),
            ArtifactKind::Closeout => safe_join(&self.mission_dir, "CLOSEOUT.md"),
            _ => Err(Codex1Error::Argument(format!(
                "{kind} is not a singleton artifact"
            ))),
        }
    }

    pub fn collection_dir(&self, kind: ArtifactKind) -> Result<PathBuf> {
        let dir = match kind {
            ArtifactKind::Research => self.research_dir(),
            ArtifactKind::Spec => self.specs_dir(),
            ArtifactKind::Subplan => self.subplans_dir().join(SubplanState::Ready.as_str()),
            ArtifactKind::Adr => self.adrs_dir(),
            ArtifactKind::Review => self.reviews_dir(),
            ArtifactKind::Triage => self.triage_dir(),
            ArtifactKind::Proof => self.proofs_dir(),
            _ => {
                return Err(Codex1Error::Argument(format!(
                    "{kind} is not a collection artifact"
                )))
            }
        };
        safe_join(
            &self.mission_dir,
            dir.strip_prefix(&self.mission_dir).unwrap_or(&dir),
        )
    }
}

fn validate_existing_mission_path(repo_root: &Path, mission_id: &str) -> Result<()> {
    let repo_real = fs::canonicalize(repo_root).io_context(format!(
        "failed to canonicalize repo root {}",
        repo_root.display()
    ))?;
    let components = [
        PathBuf::from(".codex1"),
        PathBuf::from(".codex1").join("missions"),
        PathBuf::from(".codex1").join("missions").join(mission_id),
    ];
    for relative in components {
        let path = repo_root.join(&relative);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(Codex1Error::MissionPath(format!(
                        "mission path component must not be a symlink: {}",
                        path.display()
                    )));
                }
                if !metadata.is_dir() {
                    return Err(Codex1Error::MissionPath(format!(
                        "mission path component must be a directory: {}",
                        path.display()
                    )));
                }
                let real = fs::canonicalize(&path)
                    .io_context(format!("failed to canonicalize {}", path.display()))?;
                if !real.starts_with(&repo_real) {
                    return Err(Codex1Error::MissionPath(format!(
                        "mission path escapes repo root: {}",
                        path.display()
                    )));
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(Codex1Error::Io {
                    context: format!("failed to inspect {}", path.display()),
                    source: error,
                });
            }
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct ArtifactDescriptor {
    pub kind: String,
    pub path: String,
}

pub fn descriptors(layout: &MissionLayout) -> Vec<ArtifactDescriptor> {
    ArtifactKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let path = if kind.is_singleton() {
                layout.singleton_path(kind).ok()?
            } else {
                layout.collection_dir(kind).ok()?
            };
            Some(ArtifactDescriptor {
                kind: kind.as_str().to_string(),
                path: path.display().to_string(),
            })
        })
        .chain([
            ArtifactDescriptor {
                kind: "loop-state".into(),
                path: layout.loop_file().display().to_string(),
            },
            ArtifactDescriptor {
                kind: "receipts".into(),
                path: layout.receipts_dir().display().to_string(),
            },
        ])
        .collect()
}
