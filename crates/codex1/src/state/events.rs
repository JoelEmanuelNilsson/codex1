//! Append-only EVENTS.jsonl audit log.
//!
//! One event per mutation. `seq` is monotonically increasing and matches
//! the `events_cursor` field on the post-mutation state.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::core::error::CliError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub seq: u64,
    pub at: String,
    pub kind: String,
    pub payload: Value,
}

impl Event {
    #[must_use]
    pub fn new(seq: u64, kind: impl Into<String>, payload: Value) -> Self {
        let at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
        Self {
            seq,
            at,
            kind: kind.into(),
            payload,
        }
    }
}

/// Append an event to `EVENTS.jsonl`. Assumes caller holds the state lock.
pub fn append_event(path: &Path, event: &Event) -> Result<(), CliError> {
    ensure_append_matches_tail(path, event)?;
    if last_event(path)?.is_some_and(|last| last.seq == event.seq) {
        return Ok(());
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(event)?;
    writeln!(f, "{line}")?;
    f.sync_data()?;
    Ok(())
}

/// Validate the existing tail before any precommit artifact writes.
/// Assumes caller holds the state lock.
pub fn ensure_append_matches_tail(path: &Path, event: &Event) -> Result<(), CliError> {
    if let Some(last) = last_event(path)? {
        let last_seq = last.seq;
        if last_seq == event.seq {
            if last.kind != event.kind || last.payload != event.payload {
                return Err(CliError::StateCorrupt {
                    message: format!(
                        "EVENTS.jsonl seq {} already belongs to `{}`; refusing to attach `{}` state to it",
                        event.seq, last.kind, event.kind
                    ),
                });
            }
            return Ok(());
        }
        if last_seq > event.seq {
            return Err(CliError::StateCorrupt {
                message: format!(
                    "EVENTS.jsonl already advanced to seq {last_seq}; refusing to append stale seq {}",
                    event.seq
                ),
            });
        }
    }
    Ok(())
}

fn last_event(path: &Path) -> Result<Option<Event>, CliError> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Ok(None);
    };
    let Some(line) = raw.lines().rev().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let event: Event = serde_json::from_str(line)?;
    Ok(Some(event))
}
