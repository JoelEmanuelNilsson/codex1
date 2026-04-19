//! `PROGRAM-BLUEPRINT.md` parser.
//!
//! The blueprint owns immutable route / DAG / task identity. A blueprint
//! without a DAG is **narrative only** and not executable.
//!
//! Task schema is strict: `#[serde(deny_unknown_fields)]` is applied to every
//! struct here so typos in hand-edited blueprints fail loud with
//! [`CliError::DagBadSchema`].

// T7 (graph::validate) and T11/T12 (CLI) will consume these types.
#![allow(dead_code)]

pub(crate) mod markers;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::CliError;

/// Blueprint as parsed from the YAML block. The `planning` block carries
/// meta-level fields (level, graph revision); the `tasks` vector is the DAG.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Blueprint {
    pub planning: Planning,
    #[serde(default)]
    pub tasks: Vec<TaskSpec>,
    #[serde(default)]
    pub review_boundaries: Vec<ReviewBoundary>,
}

/// Meta-level planning declaration: level and graph revision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Planning {
    pub requested_level: Level,
    #[serde(default)]
    pub risk_floor: Option<Level>,
    #[serde(default)]
    pub effective_level: Option<Level>,
    pub graph_revision: u64,
}

/// Planning intensity tier. `light | medium | hard` (no five-level bloat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Light,
    Medium,
    Hard,
}

/// Full task spec as authored in `PROGRAM-BLUEPRINT.md`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    pub id: String,
    pub title: String,
    pub kind: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_ref: Option<String>,
    #[serde(default)]
    pub read_paths: Vec<String>,
    #[serde(default)]
    pub write_paths: Vec<String>,
    #[serde(default)]
    pub exclusive_resources: Vec<String>,
    #[serde(default)]
    pub proof: Vec<String>,
    #[serde(default)]
    pub review_profiles: Vec<String>,

    // Optional side-effect declarations. Each defaults to "none"; presence
    // signals the plan author considered the side effect explicitly.
    #[serde(default)]
    pub unknown_side_effects: bool,
    #[serde(default)]
    pub package_manager_mutation: bool,
    #[serde(default)]
    pub schema_or_migration: bool,
    #[serde(default)]
    pub generated_paths: Vec<String>,
    #[serde(default)]
    pub shared_state: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub external_services: Vec<String>,
    #[serde(default)]
    pub env_mutations: Vec<String>,

    /// Replan supersession — this task replaces the listed tasks' scope.
    #[serde(default)]
    pub supersedes: Vec<String>,
}

/// Coupled-task review boundary. Wave 3 uses this; Wave 1 parses but does
/// not act on it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewBoundary {
    pub id: String,
    pub kind: String,
    pub tasks: Vec<String>,
    #[serde(default)]
    pub depends_on_clean: Vec<String>,
    #[serde(default)]
    pub requirements: Vec<ReviewRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewRequirement {
    pub id: String,
    pub profile: String,
    pub min_outputs: u32,
    #[serde(default)]
    pub allowed_roles: Vec<String>,
}

/// Read and parse `PROGRAM-BLUEPRINT.md` at `path`.
pub fn parse_blueprint(path: &Path) -> Result<Blueprint, CliError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            CliError::BlueprintInvalid {
                path: path.display().to_string(),
                reason: "file not found".into(),
                source: None,
            }
        } else {
            CliError::Io {
                path: path.display().to_string(),
                source: e,
            }
        }
    })?;
    parse_content(path, &content)
}

/// Parse already-loaded content into a Blueprint.
pub fn parse_content(path: &Path, content: &str) -> Result<Blueprint, CliError> {
    let yaml = markers::extract_block(path, content)?;
    serde_yaml::from_str::<Blueprint>(yaml).map_err(|e| {
        // serde_yaml's error message carries "unknown field" on unknown keys
        // when deny_unknown_fields fires. Surface those as DAG_BAD_SCHEMA so
        // callers can distinguish shape errors from parse errors.
        let msg = e.to_string();
        if msg.contains("unknown field") {
            CliError::DagBadSchema { reason: msg }
        } else {
            CliError::BlueprintInvalid {
                path: path.display().to_string(),
                reason: format!("YAML parse: {e}"),
                source: None,
            }
        }
    })
}

/// Round 10 P2: return the list of task ids whose raw YAML mapping
/// omits the `depends_on` key. The V2 plan contract requires every
/// task to declare its dependency graph explicitly (even `[]`); the
/// typed `Blueprint` struct defaults missing fields to empty, so
/// enforcement has to happen against the raw mapping.
pub fn tasks_missing_explicit_depends_on(path: &Path) -> Result<Vec<String>, CliError> {
    let content = std::fs::read_to_string(path).map_err(|e| CliError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let yaml = markers::extract_block(path, &content)?;
    let root: serde_yaml::Value =
        serde_yaml::from_str(yaml).map_err(|e| CliError::BlueprintInvalid {
            path: path.display().to_string(),
            reason: format!("YAML parse: {e}"),
            source: None,
        })?;
    let tasks = root.get("tasks").and_then(|t| t.as_sequence());
    let Some(tasks) = tasks else {
        return Ok(vec![]);
    };
    let mut missing: Vec<String> = Vec::new();
    for task in tasks {
        let Some(map) = task.as_mapping() else {
            continue;
        };
        let id = map
            .get(serde_yaml::Value::String("id".into()))
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>")
            .to_string();
        if !map.contains_key(serde_yaml::Value::String("depends_on".into())) {
            missing.push(id);
        }
    }
    Ok(missing)
}

#[cfg(test)]
mod tests {
    use super::markers::{END_MARKER, START_MARKER};
    use super::{Level, parse_content};
    use std::path::Path;

    fn wrap(yaml: &str) -> String {
        format!("# Program Blueprint\n\n{START_MARKER}\n{yaml}\n{END_MARKER}\n")
    }

    #[test]
    fn empty_tasks_list_is_valid() {
        let src = wrap("planning:\n  requested_level: light\n  graph_revision: 1\ntasks: []\n");
        let bp = parse_content(Path::new("/x.md"), &src).unwrap();
        assert_eq!(bp.planning.requested_level, Level::Light);
        assert!(bp.tasks.is_empty());
    }

    #[test]
    fn single_task_parses_with_required_fields() {
        let src = wrap(
            "planning:\n  requested_level: medium\n  graph_revision: 1\n\
             tasks:\n  - id: T1\n    title: Scaffold\n    kind: code\n    depends_on: []\n\
             \x20   read_paths: []\n    write_paths: []\n    exclusive_resources: []\n\
             \x20   proof: []\n    review_profiles: [local_spec_intent]\n",
        );
        let bp = parse_content(Path::new("/x.md"), &src).unwrap();
        assert_eq!(bp.tasks.len(), 1);
        let t = &bp.tasks[0];
        assert_eq!(t.id, "T1");
        assert_eq!(t.title, "Scaffold");
        assert_eq!(t.kind, "code");
        assert_eq!(t.review_profiles, vec!["local_spec_intent".to_string()]);
        assert!(!t.unknown_side_effects);
    }

    #[test]
    fn unknown_side_effects_propagates() {
        let src = wrap(
            "planning:\n  requested_level: hard\n  graph_revision: 1\n\
             tasks:\n  - id: T1\n    title: t\n    kind: code\n    unknown_side_effects: true\n",
        );
        let bp = parse_content(Path::new("/x.md"), &src).unwrap();
        assert!(bp.tasks[0].unknown_side_effects);
    }

    #[test]
    fn unknown_field_in_task_rejected() {
        let src = wrap(
            "planning:\n  requested_level: light\n  graph_revision: 1\n\
             tasks:\n  - id: T1\n    title: t\n    kind: code\n    wat: unexpected\n",
        );
        let err = parse_content(Path::new("/x.md"), &src).unwrap_err();
        assert_eq!(err.code(), "DAG_BAD_SCHEMA");
    }

    #[test]
    fn unknown_field_in_top_level_rejected() {
        let src = wrap(
            "planning:\n  requested_level: light\n  graph_revision: 1\n\
             tasks: []\nextra_top: bad\n",
        );
        let err = parse_content(Path::new("/x.md"), &src).unwrap_err();
        assert_eq!(err.code(), "DAG_BAD_SCHEMA");
    }

    #[test]
    fn missing_marker_block_errors() {
        let err = parse_content(Path::new("/x.md"), "# just prose\n").unwrap_err();
        assert_eq!(err.code(), "DAG_NO_BLOCK");
    }

    #[test]
    fn malformed_yaml_errors() {
        let src = wrap("planning: not-a-map\ntasks: []\n");
        let err = parse_content(Path::new("/x.md"), &src).unwrap_err();
        assert_eq!(err.code(), "BLUEPRINT_INVALID");
    }

    #[test]
    fn review_boundary_parses() {
        let src = wrap(
            "planning:\n  requested_level: hard\n  graph_revision: 1\n\
             tasks: []\nreview_boundaries:\n  - id: RB1\n    kind: integration\n\
             \x20   tasks: [T2, T3]\n    requirements:\n      - id: RB1-intent\n\
             \x20       profile: integration_intent\n        min_outputs: 1\n\
             \x20       allowed_roles: [reviewer]\n",
        );
        let bp = parse_content(Path::new("/x.md"), &src).unwrap();
        assert_eq!(bp.review_boundaries.len(), 1);
        assert_eq!(bp.review_boundaries[0].requirements.len(), 1);
    }

    #[test]
    fn effective_level_optional() {
        let src = wrap(
            "planning:\n  requested_level: medium\n  risk_floor: hard\n  effective_level: hard\n  graph_revision: 2\ntasks: []\n",
        );
        let bp = parse_content(Path::new("/x.md"), &src).unwrap();
        assert_eq!(bp.planning.risk_floor, Some(Level::Hard));
        assert_eq!(bp.planning.effective_level, Some(Level::Hard));
        assert_eq!(bp.planning.graph_revision, 2);
    }
}
