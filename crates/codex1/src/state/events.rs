//! Append-only EVENTS.jsonl audit log.
//!
//! One event per mutation. `seq` is monotonically increasing and matches
//! the `events_cursor` field on the post-mutation state.

use std::fs::OpenOptions;
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
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(event)?;
    writeln!(f, "{line}")?;
    f.sync_data()?;
    Ok(())
}
