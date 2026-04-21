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
use sha2::{Digest, Sha256};

use crate::core::error::CliError;
use crate::core::paths::{
    ensure_artifact_file_read_safe, ensure_artifact_file_write_safe,
    ensure_artifact_parent_write_safe, ensure_mission_write_safe, MissionPaths,
};

use self::events::{append_event, Event};
use self::fs_atomic::atomic_write;
pub use self::schema::*;

/// Enforce `--expect-revision <N>` against an already-loaded state.
///
/// Used by idempotent / dry-run short-circuits so every command path
/// that honors `--expect-revision` applies strict-equality semantics,
/// regardless of whether the call actually lands a mutation. See
/// `docs/cli-contract-schemas.md:74` and `docs/mission-anatomy.md:51`.
pub fn check_expected_revision(
    expected: Option<u64>,
    state: &MissionState,
) -> Result<(), CliError> {
    if let Some(expected) = expected {
        if expected != state.revision {
            return Err(CliError::RevisionConflict {
                expected,
                actual: state.revision,
            });
        }
    }
    Ok(())
}

/// Fail closed on work-phase mutations when the plan is not locked.
///
/// `codex1 replan record` unlocks the plan (`plan.locked = false`) and
/// the next expected action is `plan`. Task/review mutations during
/// that window can attach state to tasks whose spec has since been
/// changed (or superseded) — a state-corruption regression. Mirrors
/// the blocker at `cli/close/check.rs:84` and the short-circuit at
/// `cli/plan/waves.rs:66`.
pub fn require_plan_locked(state: &MissionState) -> Result<(), CliError> {
    if state.plan.locked {
        return Ok(());
    }
    Err(CliError::PlanInvalid {
        message: "plan is not locked; cannot mutate tasks or reviews until it is".to_string(),
        hint: Some("Run `codex1 plan check` to lock PLAN.yaml first.".to_string()),
    })
}

pub fn plan_hash(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(7 + digest.len() * 2);
    out.push_str("sha256:");
    for b in digest {
        let _ = write!(out, "{b:02x}");
    }
    out
}

pub fn require_locked_plan_snapshot(
    paths: &MissionPaths,
    state: &MissionState,
) -> Result<(), CliError> {
    require_plan_locked(state)?;
    let Some(expected_hash) = state.plan.hash.as_deref() else {
        return Ok(());
    };

    let plan_path = paths.plan();
    ensure_artifact_file_read_safe(paths, &plan_path, "PLAN.yaml")?;
    let raw = fs::read(&plan_path).map_err(|err| CliError::PlanInvalid {
        message: format!("Failed to read PLAN.yaml at {}: {err}", plan_path.display()),
        hint: Some("Run `codex1 plan check` after restoring PLAN.yaml.".to_string()),
    })?;
    let current_hash = plan_hash(&raw);
    if current_hash == expected_hash {
        return Ok(());
    }

    Err(CliError::PlanInvalid {
        message:
            "PLAN.yaml changed after the plan was locked; run `codex1 replan record` before using the updated DAG"
                .to_string(),
        hint: Some(
            "Use `codex1 replan record ...`, edit PLAN.yaml, then `codex1 plan check` to relock the new DAG."
                .to_string(),
        ),
    })
}

pub fn require_not_terminal(state: &MissionState) -> Result<(), CliError> {
    if let Some(closed_at) = state.close.terminal_at.as_ref() {
        return Err(CliError::TerminalAlreadyComplete {
            closed_at: closed_at.clone(),
        });
    }
    Ok(())
}

pub fn require_executable_plan(paths: &MissionPaths, state: &MissionState) -> Result<(), CliError> {
    require_not_terminal(state)?;
    require_locked_plan_snapshot(paths, state)?;
    if state.replan.triggered {
        return Err(CliError::PlanInvalid {
            message: "replan is required before executing more task work".to_string(),
            hint: Some(
                "Update PLAN.yaml and relock it with `codex1 plan check` before starting more tasks."
                    .to_string(),
            ),
        });
    }
    Ok(())
}

/// Read-only state load. Takes a shared fs2 lock during the read.
pub fn load(paths: &MissionPaths) -> Result<MissionState, CliError> {
    let state_path = paths.state();
    if !state_path.exists() {
        return Err(CliError::StateCorrupt {
            message: format!("STATE.json missing at {}", state_path.display()),
        });
    }
    ensure_artifact_file_read_safe(paths, &state_path, "STATE.json")?;
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
    mutate_dynamic(paths, expected_revision, |state| {
        mutator(state)?;
        Ok((event_kind.to_string(), event_payload))
    })
}

pub fn mutate_dynamic<F>(
    paths: &MissionPaths,
    expected_revision: Option<u64>,
    mutator: F,
) -> Result<Mutation, CliError>
where
    F: FnOnce(&mut MissionState) -> Result<(String, serde_json::Value), CliError>,
{
    ensure_mission_write_safe(paths)?;
    ensure_artifact_file_write_safe(paths, &paths.events(), "EVENTS.jsonl")?;
    ensure_artifact_file_write_safe(paths, &paths.state_lock(), "STATE.json.lock")?;
    let lock = acquire_exclusive_lock(paths)?;
    let state_path = paths.state();
    if !state_path.exists() {
        drop(lock);
        return Err(CliError::StateCorrupt {
            message: format!("STATE.json missing at {}", state_path.display()),
        });
    }
    ensure_artifact_file_read_safe(paths, &state_path, "STATE.json")?;
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
    let (event_kind, event_payload) = mutator(&mut state)?;
    state.revision = state.revision.saturating_add(1);
    state.events_cursor = state.events_cursor.saturating_add(1);
    // Ordering: append EVENTS.jsonl FIRST, then persist STATE.json.
    //
    // The invariant at `docs/mission-anatomy.md:62` is "the `seq` of the
    // latest line matches `state.events_cursor`". A crash between the
    // two persistent writes leaves the system in one of two shapes:
    //
    // - Events-before-state (current order): EVENTS has `seq = N+1` but
    //   STATE still reads `events_cursor = N`. An external sweep can
    //   detect "trailing JSONL line beyond events_cursor" and flag it.
    // - State-before-events (prior order): STATE claims `events_cursor
    //   = N+1` with no matching JSONL line. The audit trail is missing
    //   a mutation, which is silently wrong.
    //
    // The trailing-line shape is recoverable (the operation can be
    // retried and will re-append, keeping the audit log append-only);
    // the missing-line shape is not. Prefer the recoverable failure.
    let event = Event::new(state.events_cursor, event_kind, event_payload);
    append_event(&paths.events(), &event)?;
    let serialized = serde_json::to_vec_pretty(&state)?;
    atomic_write(&state_path, &serialized)?;
    drop(lock);
    Ok(Mutation {
        new_revision: state.revision,
        event,
        state,
    })
}

pub fn mutate_dynamic_with_precommit<F, P>(
    paths: &MissionPaths,
    expected_revision: Option<u64>,
    mutator: F,
    precommit: P,
) -> Result<Mutation, CliError>
where
    F: FnOnce(&mut MissionState) -> Result<(String, serde_json::Value), CliError>,
    P: FnOnce(&MissionState, &Event) -> Result<(), CliError>,
{
    ensure_mission_write_safe(paths)?;
    ensure_artifact_file_write_safe(paths, &paths.events(), "EVENTS.jsonl")?;
    ensure_artifact_file_write_safe(paths, &paths.state_lock(), "STATE.json.lock")?;
    let lock = acquire_exclusive_lock(paths)?;
    let state_path = paths.state();
    if !state_path.exists() {
        drop(lock);
        return Err(CliError::StateCorrupt {
            message: format!("STATE.json missing at {}", state_path.display()),
        });
    }
    ensure_artifact_file_read_safe(paths, &state_path, "STATE.json")?;
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
    let (event_kind, event_payload) = mutator(&mut state)?;
    state.revision = state.revision.saturating_add(1);
    state.events_cursor = state.events_cursor.saturating_add(1);
    let event = Event::new(state.events_cursor, event_kind, event_payload);
    precommit(&state, &event)?;
    append_event(&paths.events(), &event)?;
    let serialized = serde_json::to_vec_pretty(&state)?;
    atomic_write(&state_path, &serialized)?;
    drop(lock);
    Ok(Mutation {
        new_revision: state.revision,
        event,
        state,
    })
}

pub enum MaybeMutation {
    Mutated(Mutation),
    Unchanged(MissionState),
}

pub fn mutate_dynamic_maybe<F>(
    paths: &MissionPaths,
    expected_revision: Option<u64>,
    mutator: F,
) -> Result<MaybeMutation, CliError>
where
    F: FnOnce(&mut MissionState) -> Result<Option<(String, serde_json::Value)>, CliError>,
{
    ensure_mission_write_safe(paths)?;
    ensure_artifact_file_write_safe(paths, &paths.events(), "EVENTS.jsonl")?;
    ensure_artifact_file_write_safe(paths, &paths.state_lock(), "STATE.json.lock")?;
    let lock = acquire_exclusive_lock(paths)?;
    let state_path = paths.state();
    if !state_path.exists() {
        drop(lock);
        return Err(CliError::StateCorrupt {
            message: format!("STATE.json missing at {}", state_path.display()),
        });
    }
    ensure_artifact_file_read_safe(paths, &state_path, "STATE.json")?;
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
    let Some((event_kind, event_payload)) = mutator(&mut state)? else {
        drop(lock);
        return Ok(MaybeMutation::Unchanged(state));
    };
    state.revision = state.revision.saturating_add(1);
    state.events_cursor = state.events_cursor.saturating_add(1);
    let event = Event::new(state.events_cursor, event_kind, event_payload);
    append_event(&paths.events(), &event)?;
    let serialized = serde_json::to_vec_pretty(&state)?;
    atomic_write(&state_path, &serialized)?;
    drop(lock);
    Ok(MaybeMutation::Mutated(Mutation {
        new_revision: state.revision,
        event,
        state,
    }))
}

/// Write a fresh state for `codex1 init`. Requires the mission directory
/// to exist. Fails if STATE.json already exists (init is idempotent but
/// refuses to clobber a live mission).
pub fn init_write(paths: &MissionPaths, state: &MissionState) -> Result<(), CliError> {
    ensure_mission_write_safe(paths)?;
    ensure_artifact_file_write_safe(paths, &paths.state_lock(), "STATE.json.lock")?;
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
    ensure_mission_write_safe(paths)?;
    ensure_artifact_parent_write_safe(paths, &paths.state())?;
    ensure_artifact_parent_write_safe(paths, &paths.events())?;
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
    ensure_artifact_file_write_safe(paths, &paths.state_lock(), "STATE.json.lock")?;
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
    ensure_artifact_file_write_safe(paths, &paths.state_lock(), "STATE.json.lock")?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .read(true)
        .open(paths.state_lock())?;
    lock.lock_exclusive()?;
    Ok(lock)
}
