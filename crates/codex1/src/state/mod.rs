//! STATE.json storage: atomic reads, locked mutations, event append.
//!
//! The contract downstream units rely on:
//!
//! 1. All mutating commands call `StateStore::mutate(&paths, |state| { … })`.
//! 2. `mutate` takes an exclusive `fs2` lock on `STATE.json.lock`, reads the
//!    current state, runs the closure, bumps `revision`, atomically writes
//!    the new state, appends one event to `EVENTS.jsonl`, and releases the
//!    lock.
//! 3. `--expect-revision N` is enforced against the read state using strict
//!    equality (`REVISION_CONFLICT` on mismatch).
//! 4. Read-only callers use `StateStore::load` which acquires a shared lock.
//!
//! Atomicity: the write uses `tempfile::NamedTempFile::new_in(mission_dir)`
//! followed by `persist` (rename), guaranteeing same-filesystem atomicity.
//! Network mounts are warned against in `doctor`.

pub mod events;
pub mod fs_atomic;
pub mod readiness;
pub mod schema;

use std::fs::{self, File, OpenOptions};

use fs2::FileExt;

use crate::core::error::CliError;
use crate::core::paths::MissionPaths;

use self::events::{append_event, Event};
use self::fs_atomic::atomic_write;
pub use self::schema::*;

/// Read-only state load. Takes a shared fs2 lock during the read.
pub fn load(paths: &MissionPaths) -> Result<MissionState, CliError> {
    let state_path = paths.state();
    if !state_path.is_file() {
        return Err(CliError::StateCorrupt {
            message: format!("STATE.json missing at {}", state_path.display()),
        });
    }
    let lock = acquire_shared_lock(paths)?;
    let raw = fs::read_to_string(&state_path)?;
    drop(lock);
    serde_json::from_str(&raw).map_err(|err| CliError::StateCorrupt {
        message: format!("Failed to parse STATE.json: {err}"),
    })
}

/// Mutate the state under an exclusive lock. Returns the new revision
/// and the event that was appended to `EVENTS.jsonl`.
pub fn mutate<F>(
    paths: &MissionPaths,
    expected_revision: Option<u64>,
    event_kind: &str,
    event_payload: serde_json::Value,
    mutator: F,
) -> Result<Mutation, CliError>
where
    F: FnOnce(&mut MissionState) -> Result<(), CliError>,
{
    let lock = acquire_exclusive_lock(paths)?;
    let state_path = paths.state();
    if !state_path.is_file() {
        drop(lock);
        return Err(CliError::StateCorrupt {
            message: format!("STATE.json missing at {}", state_path.display()),
        });
    }
    let raw = fs::read_to_string(&state_path)?;
    let mut state: MissionState =
        serde_json::from_str(&raw).map_err(|err| CliError::StateCorrupt {
            message: format!("Failed to parse STATE.json: {err}"),
        })?;
    if let Some(expected) = expected_revision {
        if expected != state.revision {
            drop(lock);
            return Err(CliError::RevisionConflict {
                expected,
                actual: state.revision,
            });
        }
    }
    mutator(&mut state)?;
    state.revision = state.revision.saturating_add(1);
    state.events_cursor = state.events_cursor.saturating_add(1);
    let serialized = serde_json::to_vec_pretty(&state)?;
    atomic_write(&state_path, &serialized)?;
    let event = Event::new(state.events_cursor, event_kind, event_payload);
    append_event(&paths.events(), &event)?;
    drop(lock);
    Ok(Mutation {
        new_revision: state.revision,
        event,
        state,
    })
}

/// Write a fresh state for `codex1 init`. Requires the mission directory
/// to exist. Fails if STATE.json already exists (init is idempotent but
/// refuses to clobber a live mission).
pub fn init_write(paths: &MissionPaths, state: &MissionState) -> Result<(), CliError> {
    if paths.state().is_file() {
        return Err(CliError::StateCorrupt {
            message: format!(
                "STATE.json already exists at {}; refusing to overwrite",
                paths.state().display()
            ),
        });
    }
    std::fs::create_dir_all(&paths.mission_dir)?;
    std::fs::create_dir_all(paths.specs_dir())?;
    std::fs::create_dir_all(paths.reviews_dir())?;
    // Touch the lock file so future acquires do not race on creation.
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(paths.state_lock())?;
    atomic_write(&paths.state(), &serde_json::to_vec_pretty(state)?)?;
    // Empty EVENTS.jsonl.
    atomic_write(&paths.events(), &[])?;
    Ok(())
}

/// Result of a successful mutation.
#[derive(Debug, Clone)]
pub struct Mutation {
    pub new_revision: u64,
    pub event: Event,
    pub state: MissionState,
}

fn acquire_shared_lock(paths: &MissionPaths) -> Result<File, CliError> {
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .read(true)
        .open(paths.state_lock())?;
    lock.lock_shared()?;
    Ok(lock)
}

fn acquire_exclusive_lock(paths: &MissionPaths) -> Result<File, CliError> {
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .read(true)
        .open(paths.state_lock())?;
    lock.lock_exclusive()?;
    Ok(lock)
}
