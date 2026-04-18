//! DAG validation and the `is_eligible` predicate.
//!
//! Validation is strict:
//!
//! * IDs must match `^T[0-9]+$`.
//! * IDs are unique.
//! * Every `depends_on` target must reference an existing task.
//! * The graph must be acyclic.
//!
//! The eligibility predicate is shared between wave derivation
//! (`graph::waves`) and status projection (`status::project_status`) — both
//! read it here so `status` doesn't need to depend on `waves.rs`.

// T8, T9, and T11/T12 will call `build_dag` and `is_eligible`.
#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::blueprint::{Blueprint, TaskSpec};
use crate::error::CliError;
use crate::state::{State, TaskStatus};

use super::Dag;

/// Validate the blueprint and produce a `Dag`. Fails with the first detected
/// problem (id format → duplicates → missing deps → cycles).
pub fn build_dag(blueprint: &Blueprint) -> Result<Dag, CliError> {
    for task in &blueprint.tasks {
        validate_id_format(&task.id)?;
    }
    detect_duplicates(&blueprint.tasks)?;
    let ids: HashSet<&str> = blueprint.tasks.iter().map(|t| t.id.as_str()).collect();
    detect_missing_deps(&blueprint.tasks, &ids)?;
    detect_cycle(&blueprint.tasks)?;

    let mut tasks = BTreeMap::new();
    for t in &blueprint.tasks {
        tasks.insert(t.id.clone(), t.clone());
    }
    Ok(Dag {
        graph_revision: blueprint.planning.graph_revision,
        tasks,
    })
}

/// `^T[0-9]+$` — literal `T`, then one or more ASCII digits, nothing else.
/// No leading zeros are rejected by this predicate (e.g. `T01` is allowed at
/// this layer) because the contract only constrains the regex; authors are
/// free to use `T1`, `T10`, or `T01`. Uniqueness is checked separately.
pub fn validate_id_format(id: &str) -> Result<(), CliError> {
    let bytes = id.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'T' {
        return Err(CliError::DagBadId { got: id.to_string() });
    }
    if !bytes[1..].iter().all(u8::is_ascii_digit) {
        return Err(CliError::DagBadId { got: id.to_string() });
    }
    Ok(())
}

fn detect_duplicates(tasks: &[TaskSpec]) -> Result<(), CliError> {
    let mut seen: HashSet<&str> = HashSet::with_capacity(tasks.len());
    for t in tasks {
        if !seen.insert(t.id.as_str()) {
            return Err(CliError::DagDuplicateId { id: t.id.clone() });
        }
    }
    Ok(())
}

fn detect_missing_deps(tasks: &[TaskSpec], ids: &HashSet<&str>) -> Result<(), CliError> {
    for task in tasks {
        for dep in &task.depends_on {
            if !ids.contains(dep.as_str()) {
                return Err(CliError::DagMissingDep {
                    task: task.id.clone(),
                    missing: dep.clone(),
                });
            }
        }
    }
    Ok(())
}

fn detect_cycle(tasks: &[TaskSpec]) -> Result<(), CliError> {
    // Iterative DFS with three-colour marking: 0 = unvisited, 1 = on stack,
    // 2 = finished. A back-edge (encountering a 1-marked node) is a cycle.
    let mut color: HashMap<&str, u8> = tasks.iter().map(|t| (t.id.as_str(), 0u8)).collect();
    let adj: HashMap<&str, Vec<&str>> = tasks
        .iter()
        .map(|t| {
            (
                t.id.as_str(),
                t.depends_on.iter().map(String::as_str).collect(),
            )
        })
        .collect();

    for task in tasks {
        if color[task.id.as_str()] == 0 {
            let mut stack: Vec<(&str, usize)> = vec![(task.id.as_str(), 0)];
            let mut path: Vec<&str> = vec![task.id.as_str()];
            *color.get_mut(task.id.as_str()).unwrap() = 1;
            while let Some(&mut (node, ref mut idx)) = stack.last_mut() {
                let neighbors = &adj[node];
                if *idx < neighbors.len() {
                    let n = neighbors[*idx];
                    *idx += 1;
                    match color[n] {
                        0 => {
                            *color.get_mut(n).unwrap() = 1;
                            path.push(n);
                            stack.push((n, 0));
                        }
                        1 => {
                            // Found a cycle: slice from where `n` first appears.
                            let start = path.iter().position(|&p| p == n).unwrap_or(0);
                            let mut cycle: Vec<String> =
                                path[start..].iter().map(|s| (*s).to_string()).collect();
                            cycle.push(n.to_string());
                            return Err(CliError::DagCycle {
                                path: cycle.join(" -> "),
                                cycle,
                            });
                        }
                        _ => { /* already finished; skip */ }
                    }
                } else {
                    *color.get_mut(node).unwrap() = 2;
                    path.pop();
                    stack.pop();
                }
            }
        }
    }
    Ok(())
}

/// Is the task eligible to be scheduled into a wave?
///
/// Eligibility is the intersection of:
///
/// * task status is `Ready` or `NeedsRepair`;
/// * every `depends_on` target is `ReviewClean` or `Complete`;
/// * the task exists in the DAG (callers should only pass known ids).
///
/// This predicate does **not** check workspace-safety (write-path
/// disjointness, exclusive resources, unknown side effects) — those checks
/// live in `graph::waves` when composing a parallel wave.
#[must_use]
pub fn is_eligible(task_id: &str, state: &State, dag: &Dag) -> bool {
    let Some(spec) = dag.get(task_id) else {
        return false;
    };
    let status = state
        .tasks
        .get(task_id)
        .map_or(TaskStatus::Planned, |t| t.status);
    if !matches!(status, TaskStatus::Ready | TaskStatus::NeedsRepair) {
        return false;
    }
    spec.depends_on.iter().all(|dep| {
        state
            .tasks
            .get(dep)
            .is_some_and(|t| t.status.satisfies_dep())
    })
}

#[cfg(test)]
mod tests {
    use super::{build_dag, is_eligible, validate_id_format};
    use crate::blueprint::{Blueprint, Planning, TaskSpec};
    use crate::blueprint::Level;
    use crate::state::{ParentLoop, Phase, State, TaskState, TaskStatus};
    use std::collections::BTreeMap;

    fn planning() -> Planning {
        Planning {
            requested_level: Level::Light,
            risk_floor: None,
            effective_level: None,
            graph_revision: 1,
        }
    }

    fn t(id: &str, deps: &[&str]) -> TaskSpec {
        TaskSpec {
            id: id.into(),
            title: id.into(),
            kind: "code".into(),
            depends_on: deps.iter().map(|s| (*s).to_string()).collect(),
            spec_ref: None,
            read_paths: vec![],
            write_paths: vec![],
            exclusive_resources: vec![],
            proof: vec![],
            review_profiles: vec![],
            unknown_side_effects: false,
            package_manager_mutation: false,
            schema_or_migration: false,
            generated_paths: vec![],
            shared_state: vec![],
            commands: vec![],
            external_services: vec![],
            env_mutations: vec![],
            supersedes: vec![],
        }
    }

    #[test]
    fn valid_id_format_accepts_t1_t10_t01() {
        for id in ["T1", "T2", "T10", "T01", "T999"] {
            assert!(validate_id_format(id).is_ok(), "{id:?}");
        }
    }

    #[test]
    fn invalid_id_format_rejected() {
        for id in ["t1", "TASK1", "T", "T-1", "T1a", "", "1T"] {
            let err = validate_id_format(id).unwrap_err();
            assert_eq!(err.code(), "DAG_BAD_ID", "{id:?}");
        }
    }

    #[test]
    fn empty_dag_builds() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        assert!(dag.is_empty());
    }

    #[test]
    fn duplicate_id_rejected() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t("T1", &[]), t("T1", &[])],
            review_boundaries: vec![],
        };
        let err = build_dag(&bp).unwrap_err();
        assert_eq!(err.code(), "DAG_DUPLICATE_ID");
    }

    #[test]
    fn missing_dep_rejected() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t("T1", &["T99"])],
            review_boundaries: vec![],
        };
        let err = build_dag(&bp).unwrap_err();
        assert_eq!(err.code(), "DAG_MISSING_DEP");
        assert!(err.to_string().contains("T99"));
    }

    #[test]
    fn simple_cycle_detected() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t("T1", &["T2"]), t("T2", &["T1"])],
            review_boundaries: vec![],
        };
        let err = build_dag(&bp).unwrap_err();
        assert_eq!(err.code(), "DAG_CYCLE");
    }

    #[test]
    fn three_node_cycle_detected() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![
                t("T1", &["T2"]),
                t("T2", &["T3"]),
                t("T3", &["T1"]),
            ],
            review_boundaries: vec![],
        };
        let err = build_dag(&bp).unwrap_err();
        assert_eq!(err.code(), "DAG_CYCLE");
    }

    #[test]
    fn acyclic_graph_builds() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![
                t("T1", &[]),
                t("T2", &["T1"]),
                t("T3", &["T1"]),
                t("T4", &["T2", "T3"]),
            ],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        assert_eq!(dag.len(), 4);
        assert_eq!(dag.ids(), vec!["T1", "T2", "T3", "T4"]);
    }

    #[test]
    fn bad_id_blocks_further_checks() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t("t1", &[])],
            review_boundaries: vec![],
        };
        let err = build_dag(&bp).unwrap_err();
        assert_eq!(err.code(), "DAG_BAD_ID");
    }

    fn state_with_tasks(pairs: &[(&str, TaskStatus)]) -> State {
        let mut tasks = BTreeMap::new();
        for (id, status) in pairs {
            tasks.insert(
                (*id).to_string(),
                TaskState {
                    status: *status,
                    started_at: None,
                    finished_at: None,
                    reviewed_at: None,
                    task_run_id: None,
                    proof_ref: None,
                    proof_hash: None,
                },
            );
        }
        State {
            mission_id: "m".into(),
            state_revision: 1,
            phase: Phase::Clarify,
            parent_loop: ParentLoop::default(),
            tasks,
        }
    }

    #[test]
    fn is_eligible_true_for_ready_with_no_deps() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t("T1", &[])],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with_tasks(&[("T1", TaskStatus::Ready)]);
        assert!(is_eligible("T1", &state, &dag));
    }

    #[test]
    fn is_eligible_false_when_status_not_ready_or_needs_repair() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t("T1", &[])],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        for status in [
            TaskStatus::Planned,
            TaskStatus::InProgress,
            TaskStatus::ProofSubmitted,
            TaskStatus::ReviewOwed,
            TaskStatus::ReviewFailed,
            TaskStatus::ReplanRequired,
            TaskStatus::ReviewClean,
            TaskStatus::Complete,
            TaskStatus::Superseded,
        ] {
            let state = state_with_tasks(&[("T1", status)]);
            assert!(!is_eligible("T1", &state, &dag), "{status:?}");
        }
    }

    #[test]
    fn is_eligible_true_for_needs_repair_with_clean_dep() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t("T1", &[]), t("T2", &["T1"])],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with_tasks(&[
            ("T1", TaskStatus::ReviewClean),
            ("T2", TaskStatus::NeedsRepair),
        ]);
        assert!(is_eligible("T2", &state, &dag));
    }

    #[test]
    fn is_eligible_false_when_dep_not_satisfied() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t("T1", &[]), t("T2", &["T1"])],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with_tasks(&[
            ("T1", TaskStatus::InProgress),
            ("T2", TaskStatus::Ready),
        ]);
        assert!(!is_eligible("T2", &state, &dag));
    }

    #[test]
    fn is_eligible_false_for_unknown_task_id() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t("T1", &[])],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with_tasks(&[("T1", TaskStatus::Ready)]);
        assert!(!is_eligible("T99", &state, &dag));
    }

    #[test]
    fn is_eligible_treats_missing_state_entry_as_planned() {
        // Task exists in DAG but no entry in STATE.json.tasks yet.
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t("T1", &[])],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with_tasks(&[]);
        // Default is Planned, which is not ready → not eligible.
        assert!(!is_eligible("T1", &state, &dag));
    }
}
