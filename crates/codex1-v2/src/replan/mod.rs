//! Replan events and `REPLAN-LOG.md` append-only ledger.
//!
//! A replan event captures a mandatory-replan trigger firing. History is
//! append-only: tasks may be superseded but not erased.

#![allow(dead_code)]

pub(crate) mod triggers;

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::CliError;

pub const REPLAN_LOG_FILENAME: &str = "REPLAN-LOG.md";

/// A single replan event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReplanEvent {
    pub reason: String,
    #[serde(default)]
    pub superseded_task_ids: Vec<String>,
    #[serde(default)]
    pub new_task_ids: Vec<String>,
    pub graph_revision: u64,
    pub state_revision: u64,
    pub recorded_at: String,
}

/// Append a markdown block describing `event` to `REPLAN-LOG.md`.
///
/// Format is machine-parseable via a fenced YAML block prefixed with a
/// heading, so later replans can be enumerated with `read_log`.
pub fn append_to_log(log_path: &Path, event: &ReplanEvent) -> Result<(), CliError> {
    let yaml = serde_yaml::to_string(event).map_err(|e| CliError::Internal {
        message: format!("serialize replan event: {e}"),
    })?;
    let block = format!("## Replan {}\n\n```yaml\n{yaml}```\n\n", event.recorded_at);
    let mut existing = if log_path.exists() {
        fs::read_to_string(log_path).map_err(|e| CliError::Io {
            path: log_path.display().to_string(),
            source: e,
        })?
    } else {
        "# Replan Log\n\n".to_string()
    };
    existing.push_str(&block);
    fs::write(log_path, existing.as_bytes()).map_err(|e| CliError::Io {
        path: log_path.display().to_string(),
        source: e,
    })?;
    Ok(())
}

/// Read all replan events from the log. Tolerates missing file (returns
/// empty vec). Tolerates surrounding prose between YAML blocks.
pub fn read_log(log_path: &Path) -> Result<Vec<ReplanEvent>, CliError> {
    if !log_path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(log_path).map_err(|e| CliError::Io {
        path: log_path.display().to_string(),
        source: e,
    })?;
    let mut events = Vec::new();
    let mut rest = content.as_str();
    while let Some(start) = rest.find("```yaml\n") {
        rest = &rest[start + "```yaml\n".len()..];
        if let Some(end) = rest.find("```") {
            let yaml = &rest[..end];
            let event: ReplanEvent =
                serde_yaml::from_str(yaml).map_err(|e| CliError::Internal {
                    message: format!("parse replan log entry: {e}"),
                })?;
            events.push(event);
            rest = &rest[end + "```".len()..];
        } else {
            break;
        }
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::{ReplanEvent, append_to_log, read_log};
    use tempfile::tempdir;

    fn sample() -> ReplanEvent {
        ReplanEvent {
            reason: "six_consecutive_non_clean_reviews".into(),
            superseded_task_ids: vec!["T2".into()],
            new_task_ids: vec!["T5".into()],
            graph_revision: 2,
            state_revision: 42,
            recorded_at: "2026-04-18T10:00:00Z".into(),
        }
    }

    #[test]
    fn append_then_read_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("REPLAN-LOG.md");
        append_to_log(&path, &sample()).unwrap();
        let log = read_log(&path).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0], sample());
    }

    #[test]
    fn multiple_appends_preserved_in_order() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("REPLAN-LOG.md");
        let mut a = sample();
        a.recorded_at = "2026-04-18T10:00:00Z".into();
        let mut b = sample();
        b.reason = "write_scope_expansion".into();
        b.recorded_at = "2026-04-18T11:00:00Z".into();
        append_to_log(&path, &a).unwrap();
        append_to_log(&path, &b).unwrap();
        let log = read_log(&path).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0], a);
        assert_eq!(log[1], b);
    }

    #[test]
    fn missing_log_reads_as_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("REPLAN-LOG.md");
        assert!(read_log(&path).unwrap().is_empty());
    }
}
