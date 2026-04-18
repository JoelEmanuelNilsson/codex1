//! `StateStore` — the only code path allowed to mutate `STATE.json`.
//!
//! Every mutation takes the per-mission fs2 lock, reads the current state,
//! calls a caller-supplied closure, bumps `state_revision`, writes the state
//! atomically, then appends one event line to `events.jsonl` with
//! `seq = new state_revision`. The atomic state write is the commit point;
//! a crash between rename and append leaves events trailing by one seq,
//! which `validate` treats as a warning (not an error).

// T5 (mission/) and T11 (cli::init) are the first external call sites.
#![allow(dead_code)]

pub(crate) mod schema;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::error::CliError;
use crate::events::{append_event, Event};
use crate::fs_atomic::{atomic_write, LockedDir};

// Re-exports consumed by other modules (mission, status, graph) as Wave 1
// fills in. `ParentLoopMode`, `TaskState`, and `TaskStatus` are re-exported
// here so downstream call sites have a single import path once they land.
#[allow(unused_imports)]
pub use schema::{ParentLoop, ParentLoopMode, Phase, State, TaskState, TaskStatus};

/// Filename of the authoritative state blob inside a mission directory.
pub const STATE_FILENAME: &str = "STATE.json";
/// Filename of the audit log inside a mission directory.
pub const EVENTS_FILENAME: &str = "events.jsonl";

/// Store for a single mission's state. Cheap to construct; state is read
/// on demand.
#[derive(Debug, Clone)]
pub struct StateStore {
    mission_dir: PathBuf,
}

/// Draft of an event to be emitted as part of a mutation. `seq` and `at`
/// are filled in by the store so the one-event-per-mutation invariant is
/// mechanically enforced.
#[derive(Debug, Clone)]
pub struct EventDraft {
    kind: String,
    extra: Map<String, Value>,
}

impl EventDraft {
    #[must_use]
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            extra: Map::new(),
        }
    }

    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }

    fn into_event(self, seq: u64, at: &str) -> Event {
        Event {
            seq,
            kind: self.kind,
            at: at.to_string(),
            extra: self.extra,
        }
    }
}

impl StateStore {
    #[must_use]
    pub fn new(mission_dir: PathBuf) -> Self {
        Self { mission_dir }
    }

    #[must_use]
    pub fn mission_dir(&self) -> &Path {
        &self.mission_dir
    }

    #[must_use]
    pub fn state_path(&self) -> PathBuf {
        self.mission_dir.join(STATE_FILENAME)
    }

    #[must_use]
    pub fn events_path(&self) -> PathBuf {
        self.mission_dir.join(EVENTS_FILENAME)
    }

    /// Read `STATE.json` without holding a lock. Safe for observational reads.
    pub fn load(&self) -> Result<State, CliError> {
        let path = self.state_path();
        let bytes = std::fs::read(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CliError::MissionNotFound {
                    path: self.mission_dir.display().to_string(),
                }
            } else {
                CliError::Io {
                    path: path.display().to_string(),
                    source: e,
                }
            }
        })?;
        serde_json::from_slice::<State>(&bytes).map_err(|e| CliError::StateCorrupt {
            path: path.display().to_string(),
            reason: format!("JSON parse: {e}"),
            source: None,
        })
    }

    /// Initialise a fresh mission. Fails with [`CliError::MissionExists`] if
    /// `STATE.json` already exists in the mission directory.
    pub fn init(&self, mission_id: &str) -> Result<State, CliError> {
        std::fs::create_dir_all(&self.mission_dir).map_err(|e| CliError::Io {
            path: self.mission_dir.display().to_string(),
            source: e,
        })?;
        let _lock = self.acquire_lock()?;
        if self.state_path().exists() {
            return Err(CliError::MissionExists {
                path: self.mission_dir.display().to_string(),
            });
        }
        let state = State {
            mission_id: mission_id.to_string(),
            state_revision: 1,
            phase: Phase::Clarify,
            parent_loop: ParentLoop::default(),
            tasks: BTreeMap::new(),
        };
        self.write_state(&state)?;
        let event = Event {
            seq: 1,
            kind: "mission_initialized".into(),
            at: now_iso(),
            extra: {
                let mut m = Map::new();
                m.insert("mission_id".into(), Value::String(mission_id.into()));
                m
            },
        };
        append_event(&self.events_path(), &event).map_err(|e| CliError::Io {
            path: self.events_path().display().to_string(),
            source: e,
        })?;
        Ok(state)
    }

    /// Mutate the state under an exclusive lock. Bumps `state_revision` and
    /// appends exactly one event (with `seq == new state_revision`).
    pub fn mutate<F>(&self, f: F) -> Result<State, CliError>
    where
        F: FnOnce(&mut State) -> Result<EventDraft, CliError>,
    {
        self.mutate_checked(None, f)
    }

    /// As [`mutate`] but rejects when the current `state_revision` does not
    /// match `expected`. Wave 1 does not call this, but the plumbing is in
    /// place so Wave 2's `task start`/`task finish` inherits it.
    pub fn mutate_checked<F>(
        &self,
        expected: Option<u64>,
        f: F,
    ) -> Result<State, CliError>
    where
        F: FnOnce(&mut State) -> Result<EventDraft, CliError>,
    {
        let _lock = self.acquire_lock()?;
        let mut state = self.load()?;
        if let Some(exp) = expected
            && state.state_revision != exp
        {
            return Err(CliError::RevisionConflict {
                expected: exp,
                actual: state.state_revision,
            });
        }
        let draft = f(&mut state)?;
        state.state_revision = state
            .state_revision
            .checked_add(1)
            .ok_or_else(|| CliError::Internal {
                message: "state_revision overflow".into(),
            })?;
        self.write_state(&state)?;
        let event = draft.into_event(state.state_revision, &now_iso());
        append_event(&self.events_path(), &event).map_err(|e| CliError::Io {
            path: self.events_path().display().to_string(),
            source: e,
        })?;
        Ok(state)
    }

    fn acquire_lock(&self) -> Result<LockedDir, CliError> {
        LockedDir::acquire(&self.mission_dir).map_err(|e| CliError::Io {
            path: self.mission_dir.display().to_string(),
            source: e,
        })
    }

    fn write_state(&self, state: &State) -> Result<(), CliError> {
        let json = serde_json::to_vec_pretty(state).map_err(|e| CliError::Internal {
            message: format!("serialize state: {e}"),
        })?;
        atomic_write(&self.state_path(), &json).map_err(|e| CliError::Io {
            path: self.state_path().display().to_string(),
            source: e,
        })
    }
}

fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use super::{EventDraft, StateStore, TaskState, TaskStatus};
    use crate::events::read_events;
    use tempfile::tempdir;

    fn store_with_dir() -> (tempfile::TempDir, StateStore) {
        let dir = tempdir().unwrap();
        let store = StateStore::new(dir.path().to_path_buf());
        (dir, store)
    }

    #[test]
    fn init_creates_state_and_event() {
        let (_dir, store) = store_with_dir();
        let state = store.init("example").unwrap();
        assert_eq!(state.mission_id, "example");
        assert_eq!(state.state_revision, 1);
        assert_eq!(state.phase, super::Phase::Clarify);
        assert!(state.tasks.is_empty());

        let events = read_events(&store.events_path()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[0].kind, "mission_initialized");
        assert_eq!(events[0].extra["mission_id"], "example");
    }

    #[test]
    fn init_refuses_when_state_exists() {
        let (_dir, store) = store_with_dir();
        store.init("example").unwrap();
        let err = store.init("example").unwrap_err();
        assert_eq!(err.code(), "MISSION_EXISTS");
    }

    #[test]
    fn load_reads_back_what_init_wrote() {
        let (_dir, store) = store_with_dir();
        let written = store.init("example").unwrap();
        let read_back = store.load().unwrap();
        assert_eq!(written, read_back);
    }

    #[test]
    fn load_on_missing_mission_returns_not_found() {
        let (_dir, store) = store_with_dir();
        let err = store.load().unwrap_err();
        assert_eq!(err.code(), "MISSION_NOT_FOUND");
    }

    #[test]
    fn mutate_bumps_revision_and_appends_event() {
        let (_dir, store) = store_with_dir();
        store.init("example").unwrap();
        let after = store
            .mutate(|state| {
                state
                    .tasks
                    .insert("T1".into(), TaskState::planned());
                Ok(EventDraft::new("task_added").with("task_id", "T1"))
            })
            .unwrap();
        assert_eq!(after.state_revision, 2);
        assert_eq!(after.tasks.len(), 1);
        assert_eq!(after.tasks["T1"].status, TaskStatus::Planned);

        let events = read_events(&store.events_path()).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].seq, 2);
        assert_eq!(events[1].kind, "task_added");
        assert_eq!(events[1].extra["task_id"], "T1");
    }

    #[test]
    fn mutate_checked_rejects_revision_mismatch() {
        let (_dir, store) = store_with_dir();
        store.init("example").unwrap();
        let err = store
            .mutate_checked(Some(999), |_| {
                Ok(EventDraft::new("never"))
            })
            .unwrap_err();
        assert_eq!(err.code(), "REVISION_CONFLICT");
        assert_eq!(err.exit_code(), 4);
        assert!(err.retryable());
    }

    #[test]
    fn mutate_propagates_closure_error_and_does_not_mutate() {
        let (_dir, store) = store_with_dir();
        store.init("example").unwrap();
        let result = store.mutate(|_state| {
            Err(crate::error::CliError::Internal {
                message: "closure refused".into(),
            })
        });
        assert!(result.is_err());
        // State stayed at revision 1.
        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.state_revision, 1);
        let events = read_events(&store.events_path()).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn last_event_seq_matches_state_revision_after_mutations() {
        let (_dir, store) = store_with_dir();
        store.init("example").unwrap();
        for i in 0..3 {
            store
                .mutate(|_state| {
                    Ok(EventDraft::new("tick").with("n", i64::from(i)))
                })
                .unwrap();
        }
        let state = store.load().unwrap();
        let events = read_events(&store.events_path()).unwrap();
        assert_eq!(state.state_revision, 4);
        assert_eq!(events.last().unwrap().seq, 4);
    }
}
