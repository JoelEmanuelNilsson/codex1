use std::path::{Component, Path, PathBuf};

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionPaths {
    repo_root: PathBuf,
    mission_id: String,
}

impl MissionPaths {
    pub fn try_new(repo_root: impl Into<PathBuf>, mission_id: impl Into<String>) -> Result<Self> {
        let mission_id = mission_id.into();
        validate_id_component("mission_id", &mission_id)?;
        Ok(Self {
            repo_root: repo_root.into(),
            mission_id,
        })
    }

    #[must_use]
    pub fn new(repo_root: impl Into<PathBuf>, mission_id: impl Into<String>) -> Self {
        Self::try_new(repo_root, mission_id).expect("mission_id must be a safe path component")
    }

    #[must_use]
    pub fn repo_root(&self) -> &PathBuf {
        &self.repo_root
    }

    #[must_use]
    pub fn mission_id(&self) -> &str {
        &self.mission_id
    }

    #[must_use]
    pub fn plans_root(&self) -> PathBuf {
        self.repo_root.join("PLANS")
    }

    #[must_use]
    pub fn mission_root(&self) -> PathBuf {
        self.plans_root().join(&self.mission_id)
    }

    #[must_use]
    pub fn readme(&self) -> PathBuf {
        self.mission_root().join("README.md")
    }

    #[must_use]
    pub fn mission_state(&self) -> PathBuf {
        self.mission_root().join("MISSION-STATE.md")
    }

    #[must_use]
    pub fn outcome_lock(&self) -> PathBuf {
        self.mission_root().join("OUTCOME-LOCK.md")
    }

    #[must_use]
    pub fn program_blueprint(&self) -> PathBuf {
        self.mission_root().join("PROGRAM-BLUEPRINT.md")
    }

    #[must_use]
    pub fn blueprint_dir(&self) -> PathBuf {
        self.mission_root().join("blueprint")
    }

    #[must_use]
    pub fn review_ledger(&self) -> PathBuf {
        self.mission_root().join("REVIEW-LEDGER.md")
    }

    #[must_use]
    pub fn replan_log(&self) -> PathBuf {
        self.mission_root().join("REPLAN-LOG.md")
    }

    #[must_use]
    pub fn child_missions_root(&self) -> PathBuf {
        self.mission_root().join("missions")
    }

    #[must_use]
    pub fn specs_root(&self) -> PathBuf {
        self.mission_root().join("specs")
    }

    #[must_use]
    pub fn spec_root(&self, spec_id: &str) -> PathBuf {
        self.specs_root()
            .join(checked_id_component("spec_id", spec_id))
    }

    #[must_use]
    pub fn spec_file(&self, spec_id: &str) -> PathBuf {
        self.spec_root(spec_id).join("SPEC.md")
    }

    #[must_use]
    pub fn review_file(&self, spec_id: &str) -> PathBuf {
        self.spec_root(spec_id).join("REVIEW.md")
    }

    #[must_use]
    pub fn notes_file(&self, spec_id: &str) -> PathBuf {
        self.spec_root(spec_id).join("NOTES.md")
    }

    #[must_use]
    pub fn receipts_dir(&self, spec_id: &str) -> PathBuf {
        self.spec_root(spec_id).join("RECEIPTS")
    }

    #[must_use]
    pub fn ralph_root(&self) -> PathBuf {
        self.repo_root.join(".ralph")
    }

    #[must_use]
    pub fn missions_root(&self) -> PathBuf {
        self.ralph_root().join("missions")
    }

    #[must_use]
    pub fn hidden_mission_root(&self) -> PathBuf {
        self.missions_root().join(&self.mission_id)
    }

    #[must_use]
    pub fn active_cycle(&self) -> PathBuf {
        self.hidden_mission_root().join("active-cycle.json")
    }

    #[must_use]
    pub fn state_json(&self) -> PathBuf {
        self.hidden_mission_root().join("state.json")
    }

    #[must_use]
    pub fn closeouts_ndjson(&self) -> PathBuf {
        self.hidden_mission_root().join("closeouts.ndjson")
    }

    #[must_use]
    pub fn contradictions_ndjson(&self) -> PathBuf {
        self.hidden_mission_root().join("contradictions.ndjson")
    }

    #[must_use]
    pub fn execution_packages_dir(&self) -> PathBuf {
        self.hidden_mission_root().join("execution-packages")
    }

    #[must_use]
    pub fn waves_dir(&self) -> PathBuf {
        self.hidden_mission_root().join("waves")
    }

    #[must_use]
    pub fn execution_package(&self, package_id: &str) -> PathBuf {
        self.execution_packages_dir().join(format!(
            "{}.json",
            checked_id_component("package_id", package_id)
        ))
    }

    #[must_use]
    pub fn wave_manifest(&self, wave_id: &str) -> PathBuf {
        self.waves_dir()
            .join(format!("{}.json", checked_id_component("wave_id", wave_id)))
    }

    #[must_use]
    pub fn receipts_root(&self) -> PathBuf {
        self.hidden_mission_root().join("receipts")
    }

    #[must_use]
    pub fn packets_dir(&self) -> PathBuf {
        self.hidden_mission_root().join("packets")
    }

    #[must_use]
    pub fn writer_packet(&self, packet_id: &str) -> PathBuf {
        self.packets_dir().join(format!(
            "{}.json",
            checked_id_component("packet_id", packet_id)
        ))
    }

    #[must_use]
    pub fn bundles_dir(&self) -> PathBuf {
        self.hidden_mission_root().join("bundles")
    }

    #[must_use]
    pub fn review_bundle(&self, bundle_id: &str) -> PathBuf {
        self.bundles_dir().join(format!(
            "{}.json",
            checked_id_component("bundle_id", bundle_id)
        ))
    }

    #[must_use]
    pub fn review_evidence_snapshots_dir(&self) -> PathBuf {
        self.hidden_mission_root().join("review-evidence-snapshots")
    }

    #[must_use]
    pub fn review_evidence_snapshot(&self, bundle_id: &str) -> PathBuf {
        self.review_evidence_snapshots_dir().join(format!(
            "{}.json",
            checked_id_component("bundle_id", bundle_id)
        ))
    }

    #[must_use]
    pub fn review_truth_snapshots_dir(&self) -> PathBuf {
        self.hidden_mission_root().join("review-truth-snapshots")
    }

    #[must_use]
    pub fn review_truth_snapshot(&self, bundle_id: &str) -> PathBuf {
        self.review_truth_snapshots_dir().join(format!(
            "{}.json",
            checked_id_component("bundle_id", bundle_id)
        ))
    }

    #[must_use]
    pub fn reviewer_outputs_dir(&self) -> PathBuf {
        self.hidden_mission_root().join("reviewer-outputs")
    }

    #[must_use]
    pub fn reviewer_outputs_for_bundle_dir(&self, bundle_id: &str) -> PathBuf {
        self.reviewer_outputs_dir()
            .join(checked_id_component("bundle_id", bundle_id))
    }

    #[must_use]
    pub fn reviewer_output(&self, bundle_id: &str, output_id: &str) -> PathBuf {
        self.reviewer_outputs_for_bundle_dir(bundle_id)
            .join(format!(
                "{}.json",
                checked_id_component("reviewer_output_id", output_id)
            ))
    }

    #[must_use]
    pub fn execution_graph(&self) -> PathBuf {
        self.hidden_mission_root().join("execution-graph.json")
    }

    #[must_use]
    pub fn gates_json(&self) -> PathBuf {
        self.hidden_mission_root().join("gates.json")
    }

    #[must_use]
    pub fn selection_state(&self) -> PathBuf {
        self.ralph_root().join("selection-state.json")
    }

    #[must_use]
    pub fn loop_lease(&self) -> PathBuf {
        self.ralph_root().join("loop-lease.json")
    }
}

fn checked_id_component<'a>(label: &'static str, raw: &'a str) -> &'a str {
    validate_id_component(label, raw).expect("path identifier must be a safe single component");
    raw
}

pub fn validate_id_component(label: &'static str, raw: &str) -> Result<()> {
    if raw.trim().is_empty() {
        return Err(CoreError::Validation(format!("{label} must not be empty")));
    }

    let path = Path::new(raw);
    if path.is_absolute() {
        return Err(CoreError::Validation(format!(
            "{label} must not be an absolute path"
        )));
    }

    let mut saw_normal = false;
    for component in path.components() {
        match component {
            Component::Normal(_) => {
                saw_normal = true;
            }
            Component::CurDir | Component::ParentDir => {
                return Err(CoreError::Validation(format!(
                    "{label} must not contain navigation segments"
                )));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(CoreError::Validation(format!(
                    "{label} must not contain absolute path segments"
                )));
            }
        }
    }

    if !saw_normal || path.components().count() != 1 {
        return Err(CoreError::Validation(format!(
            "{label} must be a single safe path component"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{MissionPaths, validate_id_component};

    #[test]
    fn resolves_expected_paths() {
        let paths = MissionPaths::new("/repo", "mission-123");

        assert_eq!(paths.repo_root(), &PathBuf::from("/repo"));
        assert_eq!(
            paths.mission_root(),
            PathBuf::from("/repo/PLANS/mission-123")
        );
        assert_eq!(
            paths.spec_file("spec-a"),
            PathBuf::from("/repo/PLANS/mission-123/specs/spec-a/SPEC.md")
        );
        assert_eq!(
            paths.closeouts_ndjson(),
            PathBuf::from("/repo/.ralph/missions/mission-123/closeouts.ndjson")
        );
        assert_eq!(
            paths.selection_state(),
            PathBuf::from("/repo/.ralph/selection-state.json")
        );
    }

    #[test]
    fn rejects_invalid_mission_ids() {
        for mission_id in ["", "../escape", "nested/mission", "/abs"] {
            assert!(MissionPaths::try_new("/repo", mission_id).is_err());
        }
    }

    #[test]
    fn rejects_invalid_child_ids() {
        for raw in ["../escape", "nested/spec", "/abs", ""] {
            assert!(validate_id_component("spec_id", raw).is_err());
        }
    }
}
