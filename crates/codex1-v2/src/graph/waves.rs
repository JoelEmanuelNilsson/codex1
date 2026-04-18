//! Wave derivation: from `(Dag, State)` compute the currently-runnable wave
//! plus a list of blocked tasks.
//!
//! Semantics:
//!
//! * A "wave" is the set of tasks that are schedulable *right now* given the
//!   current state. Projection past the current wave would require guessing
//!   future review outcomes and is explicitly out of scope.
//! * A task is "eligible" via [`super::validate::is_eligible`] (status is
//!   `Ready` or `NeedsRepair` and all deps are clean/complete).
//! * The eligible set forms a single wave. Parallel-safety is determined by
//!   pairwise checks; any failure forces `mode: serial` and records the
//!   offending flag so the caller can explain.
//! * `Ready` or `NeedsRepair` tasks with unsatisfied deps land in `blocked`.

// T12 (`cli::plan waves`, `cli::task next`) is the first non-test caller.
#![allow(dead_code)]

use std::collections::BTreeSet;

use serde::Serialize;

use crate::blueprint::TaskSpec;
use crate::state::{State, TaskStatus};

use super::validate::is_eligible;
use super::Dag;

/// Output of [`derive_waves`]. Lists deterministic, task ids ascending.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Waves {
    pub waves: Vec<Wave>,
    pub blocked: Vec<Blocked>,
}

/// A single wave. Wave 1 semantics: at most one wave is returned per call.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Wave {
    pub id: String,
    pub tasks: Vec<String>,
    pub mode: WaveMode,
    pub safety: WaveSafety,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WaveMode {
    Serial,
    Parallel,
}

/// Per-wave workspace-safety verdict. Each flag is `true` when the check
/// passes. `unknown_side_effects` is inverted — `true` means "at least one
/// task declared unknown side effects" — so it *fires* serial.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // These fields mirror the V2 status envelope contract.
pub struct WaveSafety {
    pub dependency_independent: bool,
    pub write_paths_disjoint: bool,
    pub read_write_conflicts: Vec<ReadWriteConflict>,
    pub exclusive_resources_disjoint: bool,
    pub unknown_side_effects: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReadWriteConflict {
    pub reader: String,
    pub writer: String,
    pub read_path: String,
    pub write_path: String,
}

/// Task in Ready/NeedsRepair whose dependencies are not yet clean.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Blocked {
    pub task_id: String,
    pub blocked_by: Vec<String>,
}

/// Derive the current wave plus blocked tasks.
#[must_use]
pub fn derive_waves(dag: &Dag, state: &State) -> Waves {
    let mut eligible: Vec<&TaskSpec> = dag
        .tasks
        .values()
        .filter(|t| is_eligible(&t.id, state, dag))
        .collect();
    eligible.sort_by(|a, b| a.id.cmp(&b.id));

    let mut blocked: Vec<Blocked> = dag
        .tasks
        .values()
        .filter_map(|t| blocked_entry(t, state))
        .collect();
    blocked.sort_by(|a, b| a.task_id.cmp(&b.task_id));

    let waves = if eligible.is_empty() {
        Vec::new()
    } else {
        vec![build_wave(&eligible)]
    };

    Waves { waves, blocked }
}

fn blocked_entry(task: &TaskSpec, state: &State) -> Option<Blocked> {
    let status = state
        .tasks
        .get(&task.id)
        .map_or(TaskStatus::Planned, |t| t.status);
    if !matches!(status, TaskStatus::Ready | TaskStatus::NeedsRepair) {
        return None;
    }
    let blocking: Vec<String> = task
        .depends_on
        .iter()
        .filter(|dep| {
            state
                .tasks
                .get(dep.as_str())
                .is_none_or(|t| !t.status.satisfies_dep())
        })
        .cloned()
        .collect();
    if blocking.is_empty() {
        None
    } else {
        Some(Blocked {
            task_id: task.id.clone(),
            blocked_by: blocking,
        })
    }
}

fn build_wave(tasks: &[&TaskSpec]) -> Wave {
    let ids: Vec<String> = tasks.iter().map(|t| t.id.clone()).collect();
    let safety = evaluate_safety(tasks);
    let mode = if tasks.len() == 1 || !is_parallel_safe(&safety) {
        WaveMode::Serial
    } else {
        WaveMode::Parallel
    };
    Wave {
        id: "W1".into(),
        tasks: ids,
        mode,
        safety,
    }
}

fn is_parallel_safe(safety: &WaveSafety) -> bool {
    safety.dependency_independent
        && safety.write_paths_disjoint
        && safety.read_write_conflicts.is_empty()
        && safety.exclusive_resources_disjoint
        && !safety.unknown_side_effects
}

fn evaluate_safety(tasks: &[&TaskSpec]) -> WaveSafety {
    let dep_indep = has_no_internal_edges(tasks);
    let writes_disjoint = pairwise_writes_disjoint(tasks);
    let rw_conflicts = collect_read_write_conflicts(tasks);
    let exclusive_disjoint = pairwise_exclusive_resources_disjoint(tasks);
    let unknown = tasks.iter().any(|t| t.unknown_side_effects);
    WaveSafety {
        dependency_independent: dep_indep,
        write_paths_disjoint: writes_disjoint,
        read_write_conflicts: rw_conflicts,
        exclusive_resources_disjoint: exclusive_disjoint,
        unknown_side_effects: unknown,
    }
}

fn has_no_internal_edges(tasks: &[&TaskSpec]) -> bool {
    let ids: BTreeSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    tasks
        .iter()
        .all(|t| t.depends_on.iter().all(|d| !ids.contains(d.as_str())))
}

fn pairwise_writes_disjoint(tasks: &[&TaskSpec]) -> bool {
    for (i, first) in tasks.iter().enumerate() {
        for second in tasks.iter().skip(i + 1) {
            for w_i in &first.write_paths {
                for w_j in &second.write_paths {
                    if paths_overlap(w_i, w_j) {
                        return false;
                    }
                }
            }
        }
    }
    true
}

fn collect_read_write_conflicts(tasks: &[&TaskSpec]) -> Vec<ReadWriteConflict> {
    let mut out = Vec::new();
    for reader in tasks {
        for writer in tasks {
            if reader.id == writer.id {
                continue;
            }
            for r in &reader.read_paths {
                for w in &writer.write_paths {
                    if paths_overlap(r, w) {
                        out.push(ReadWriteConflict {
                            reader: reader.id.clone(),
                            writer: writer.id.clone(),
                            read_path: r.clone(),
                            write_path: w.clone(),
                        });
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| {
        (&a.reader, &a.writer, &a.read_path, &a.write_path).cmp(&(
            &b.reader,
            &b.writer,
            &b.read_path,
            &b.write_path,
        ))
    });
    out
}

fn pairwise_exclusive_resources_disjoint(tasks: &[&TaskSpec]) -> bool {
    for (i, first) in tasks.iter().enumerate() {
        let a: BTreeSet<&str> = first.exclusive_resources.iter().map(String::as_str).collect();
        for second in tasks.iter().skip(i + 1) {
            for r in &second.exclusive_resources {
                if a.contains(r.as_str()) {
                    return false;
                }
            }
        }
    }
    true
}

/// Two path globs overlap if, after trimming trailing glob wildcards, one's
/// directory prefix is a prefix of the other's. Wave 1 uses a deliberately
/// simple heuristic — no full glob-expansion engine. Authors are expected
/// to use prefix-based globs (`src/foo/**`, `src/`, `src/bar/file.rs`).
fn paths_overlap(a: &str, b: &str) -> bool {
    let a = normalize_glob(a);
    let b = normalize_glob(b);
    a == b || has_dir_prefix(&a, &b) || has_dir_prefix(&b, &a)
}

fn normalize_glob(p: &str) -> String {
    let mut s = p.trim().to_string();
    for suffix in ["/**", "/*", "**", "*"] {
        if let Some(stripped) = s.strip_suffix(suffix) {
            s = stripped.to_string();
        }
    }
    // Ensure trailing slash so prefix comparison is meaningful for dir paths.
    if !s.is_empty() && !s.ends_with('/') {
        // Only append '/' if the path looks like a directory (no file
        // extension). Heuristic: path with a '.' after the last '/' is a file.
        let after_slash = s.rsplit_once('/').map_or(s.as_str(), |(_, tail)| tail);
        if !after_slash.contains('.') {
            s.push('/');
        }
    }
    s
}

fn has_dir_prefix(parent: &str, child: &str) -> bool {
    // `parent` must end with `/` and `child` must start with `parent`.
    if parent.is_empty() {
        return false;
    }
    if parent.ends_with('/') {
        child.starts_with(parent)
    } else {
        // File path acting as parent: equal only.
        parent == child
    }
}

#[cfg(test)]
mod tests {
    use super::{derive_waves, paths_overlap, WaveMode};
    use crate::blueprint::{Blueprint, Level, Planning, TaskSpec};
    use crate::graph::validate::build_dag;
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

    fn task(id: &str) -> TaskSpec {
        TaskSpec {
            id: id.into(),
            title: id.into(),
            kind: "code".into(),
            depends_on: vec![],
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

    fn state_with(tasks: &[(&str, TaskStatus)]) -> State {
        let mut map = BTreeMap::new();
        for (id, status) in tasks {
            map.insert(
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
            tasks: map,
        }
    }

    #[test]
    fn empty_dag_produces_no_waves() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with(&[]);
        let w = derive_waves(&dag, &state);
        assert!(w.waves.is_empty());
        assert!(w.blocked.is_empty());
    }

    #[test]
    fn single_ready_task_is_serial_wave() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![task("T1")],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with(&[("T1", TaskStatus::Ready)]);
        let w = derive_waves(&dag, &state);
        assert_eq!(w.waves.len(), 1);
        assert_eq!(w.waves[0].tasks, vec!["T1".to_string()]);
        assert_eq!(w.waves[0].mode, WaveMode::Serial);
        assert!(w.blocked.is_empty());
    }

    #[test]
    fn two_independent_disjoint_writes_run_parallel() {
        let mut t1 = task("T1");
        let mut t2 = task("T2");
        t1.write_paths = vec!["src/a/**".into()];
        t2.write_paths = vec!["src/b/**".into()];
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t1, t2],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with(&[
            ("T1", TaskStatus::Ready),
            ("T2", TaskStatus::Ready),
        ]);
        let w = derive_waves(&dag, &state);
        assert_eq!(w.waves[0].mode, WaveMode::Parallel);
        assert!(w.waves[0].safety.write_paths_disjoint);
        assert!(w.waves[0].safety.read_write_conflicts.is_empty());
        assert!(w.waves[0].safety.exclusive_resources_disjoint);
        assert!(!w.waves[0].safety.unknown_side_effects);
    }

    #[test]
    fn overlapping_writes_force_serial() {
        let mut t1 = task("T1");
        let mut t2 = task("T2");
        t1.write_paths = vec!["src/**".into()];
        t2.write_paths = vec!["src/foo/**".into()];
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t1, t2],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with(&[
            ("T1", TaskStatus::Ready),
            ("T2", TaskStatus::Ready),
        ]);
        let w = derive_waves(&dag, &state);
        assert_eq!(w.waves[0].mode, WaveMode::Serial);
        assert!(!w.waves[0].safety.write_paths_disjoint);
    }

    #[test]
    fn read_write_conflict_forces_serial_and_records_pair() {
        let mut t1 = task("T1");
        let mut t2 = task("T2");
        t1.read_paths = vec!["src/foo/**".into()];
        t2.write_paths = vec!["src/foo/bar.rs".into()];
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t1, t2],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with(&[
            ("T1", TaskStatus::Ready),
            ("T2", TaskStatus::Ready),
        ]);
        let w = derive_waves(&dag, &state);
        assert_eq!(w.waves[0].mode, WaveMode::Serial);
        assert!(!w.waves[0].safety.read_write_conflicts.is_empty());
        let c = &w.waves[0].safety.read_write_conflicts[0];
        assert_eq!(c.reader, "T1");
        assert_eq!(c.writer, "T2");
    }

    #[test]
    fn unknown_side_effects_force_serial() {
        let mut t1 = task("T1");
        let mut t2 = task("T2");
        t1.write_paths = vec!["src/a/**".into()];
        t2.write_paths = vec!["src/b/**".into()];
        t1.unknown_side_effects = true;
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t1, t2],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with(&[
            ("T1", TaskStatus::Ready),
            ("T2", TaskStatus::Ready),
        ]);
        let w = derive_waves(&dag, &state);
        assert_eq!(w.waves[0].mode, WaveMode::Serial);
        assert!(w.waves[0].safety.unknown_side_effects);
    }

    #[test]
    fn shared_exclusive_resources_force_serial() {
        let mut t1 = task("T1");
        let mut t2 = task("T2");
        t1.write_paths = vec!["src/a/**".into()];
        t2.write_paths = vec!["src/b/**".into()];
        t1.exclusive_resources = vec!["shared_thing".into()];
        t2.exclusive_resources = vec!["shared_thing".into()];
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t1, t2],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with(&[
            ("T1", TaskStatus::Ready),
            ("T2", TaskStatus::Ready),
        ]);
        let w = derive_waves(&dag, &state);
        assert_eq!(w.waves[0].mode, WaveMode::Serial);
        assert!(!w.waves[0].safety.exclusive_resources_disjoint);
    }

    #[test]
    fn dep_not_clean_moves_task_to_blocked() {
        let mut t2 = task("T2");
        t2.depends_on = vec!["T1".into()];
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![task("T1"), t2],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with(&[
            ("T1", TaskStatus::InProgress),
            ("T2", TaskStatus::Ready),
        ]);
        let w = derive_waves(&dag, &state);
        assert!(w.waves.is_empty());
        assert_eq!(w.blocked.len(), 1);
        assert_eq!(w.blocked[0].task_id, "T2");
        assert_eq!(w.blocked[0].blocked_by, vec!["T1".to_string()]);
    }

    #[test]
    fn planned_tasks_do_not_appear_in_blocked() {
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![task("T1")],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with(&[("T1", TaskStatus::Planned)]);
        let w = derive_waves(&dag, &state);
        assert!(w.waves.is_empty());
        assert!(w.blocked.is_empty()); // Planned needs a separate state transition
    }

    #[test]
    fn tasks_sorted_deterministically_in_wave() {
        let mut t1 = task("T10");
        let mut t2 = task("T2");
        t1.write_paths = vec!["a/**".into()];
        t2.write_paths = vec!["b/**".into()];
        let bp = Blueprint {
            planning: planning(),
            tasks: vec![t1, t2],
            review_boundaries: vec![],
        };
        let dag = build_dag(&bp).unwrap();
        let state = state_with(&[
            ("T10", TaskStatus::Ready),
            ("T2", TaskStatus::Ready),
        ]);
        let w = derive_waves(&dag, &state);
        // Lexicographic sort: "T10" < "T2" because '1' < '2'.
        assert_eq!(
            w.waves[0].tasks,
            vec!["T10".to_string(), "T2".to_string()]
        );
    }

    #[test]
    fn paths_overlap_heuristic_examples() {
        assert!(paths_overlap("src/**", "src/foo/**"));
        assert!(paths_overlap("src/foo/**", "src/**"));
        assert!(paths_overlap("src/foo", "src/foo"));
        assert!(paths_overlap("src/foo/bar.rs", "src/foo/bar.rs"));
        assert!(paths_overlap("src/foo/**", "src/foo/bar.rs"));
        assert!(!paths_overlap("src/foo/**", "src/bar/**"));
        assert!(!paths_overlap("src/foo/a.rs", "src/foo/b.rs"));
        assert!(!paths_overlap("Cargo.toml", "Cargo.lock"));
    }
}
