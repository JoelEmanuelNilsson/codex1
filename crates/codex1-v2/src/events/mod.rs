//! `events.jsonl` audit log — append-only, best-effort.
//!
//! Events are **not** operational truth. `STATE.json` is authoritative; every
//! mutation commits state first and then appends an event line. A crash
//! between rename and append leaves `events.jsonl` trailing by one `seq`;
//! `validate` tolerates `last event seq ≤ state_revision` (strict greater
//! than is an error).

// Call sites live in T4 (`state::StateStore::mutate`) and T11 (`init`); until
// those land the non-test callers are in this module's tests.
#![allow(dead_code)]

use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One event line in `events.jsonl`.
///
/// `seq` is the single counter shared with `STATE.json::state_revision`.
/// Extra fields are serialized flat so the JSON shape matches the contract:
/// `{"seq":1,"kind":"mission_initialized","mission_id":"…","at":"…"}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub seq: u64,
    pub kind: String,
    pub at: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Event {
    /// Build an event with an arbitrary extra payload.
    pub fn new(seq: u64, kind: impl Into<String>, at: impl Into<String>) -> Self {
        Self {
            seq,
            kind: kind.into(),
            at: at.into(),
            extra: Map::new(),
        }
    }

    /// Attach a key/value to the flattened payload.
    pub fn with(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

/// Append one event to `events_path`. Creates the file if missing. Writes
/// a single JSON-encoded line terminated with `\n`, then fsyncs the file.
///
/// Caller holds the mission's `.state.lock` so concurrent appends are
/// serialized at the process boundary.
pub fn append_event(events_path: &Path, event: &Event) -> io::Result<()> {
    let line = serde_json::to_string(event)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(events_path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

/// Read all events from `events_path` in order. Empty lines are skipped.
/// Returns an empty vector if the file does not exist.
pub fn read_events(events_path: &Path) -> io::Result<Vec<Event>> {
    if !events_path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(events_path)?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let event: Event = serde_json::from_str(&line).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("events.jsonl line {} is not a valid event: {e}", idx + 1),
            )
        })?;
        events.push(event);
    }
    Ok(events)
}

/// Last recorded `seq` or `None` if the log is empty/missing.
pub fn last_seq(events_path: &Path) -> io::Result<Option<u64>> {
    Ok(read_events(events_path)?.last().map(|e| e.seq))
}

#[cfg(test)]
mod tests {
    use super::{append_event, last_seq, read_events, Event};
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn empty_events_file_reads_as_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        assert!(read_events(&path).unwrap().is_empty());
        assert!(last_seq(&path).unwrap().is_none());
    }

    #[test]
    fn append_then_read_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let event = Event::new(1, "mission_initialized", "2026-04-18T10:00:00Z")
            .with("mission_id", "example")
            .with("title", "Smoke");
        append_event(&path, &event).unwrap();
        let read_back = read_events(&path).unwrap();
        assert_eq!(read_back.len(), 1);
        assert_eq!(read_back[0], event);
        assert_eq!(read_back[0].extra["mission_id"], json!("example"));
    }

    #[test]
    fn jsonl_is_one_object_per_line() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        for seq in 1..=3 {
            append_event(
                &path,
                &Event::new(seq, "tick", "2026-04-18T10:00:00Z"),
            )
            .unwrap();
        }
        let raw = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = raw.lines().collect();
        assert_eq!(lines.len(), 3);
        for line in &lines {
            assert!(line.starts_with('{') && line.ends_with('}'));
        }
    }

    #[test]
    fn last_seq_tracks_latest_event() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        append_event(&path, &Event::new(1, "a", "2026-04-18T10:00:00Z")).unwrap();
        append_event(&path, &Event::new(2, "b", "2026-04-18T10:00:01Z")).unwrap();
        assert_eq!(last_seq(&path).unwrap(), Some(2));
    }

    #[test]
    fn read_events_rejects_malformed_lines() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        fs::write(&path, "not json\n").unwrap();
        let err = read_events(&path).unwrap_err();
        assert!(err.to_string().contains("line 1"));
    }

    #[test]
    fn blank_lines_are_skipped() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let line = serde_json::to_string(&Event::new(1, "a", "2026-04-18T10:00:00Z")).unwrap();
        fs::write(&path, format!("\n{line}\n\n")).unwrap();
        let events = read_events(&path).unwrap();
        assert_eq!(events.len(), 1);
    }
}
