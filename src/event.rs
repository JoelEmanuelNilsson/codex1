use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};

use crate::error::{Codex1Error, IoContext, Result};
use crate::layout::{ArtifactKind, MissionLayout, SubplanState};
use crate::paths::{
    create_dir_all_contained, ensure_contained_for_write, ensure_existing_contained,
};

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    MissionInitialized,
    ArtifactWritten,
    ArtifactWriteFailed,
    SubplanMoved,
    SubplanMoveFailed,
    ReceiptAppended,
    ReceiptAppendFailed,
    LoopStarted,
    LoopStartFailed,
    LoopPaused,
    LoopPauseFailed,
    LoopResumed,
    LoopResumeFailed,
    LoopStopped,
    LoopStopFailed,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventResult {
    Success,
    Error,
}

#[derive(Debug, Serialize)]
pub struct EventRecord {
    pub version: u32,
    pub timestamp: DateTime<Utc>,
    pub mission_id: String,
    pub command: &'static str,
    pub kind: EventKind,
    pub result: EventResult,
    pub duration_ms: u64,
    pub metadata: Value,
}

impl EventRecord {
    pub fn new(
        layout: &MissionLayout,
        command: &'static str,
        kind: EventKind,
        result: EventResult,
        duration: Duration,
        metadata: Value,
    ) -> Self {
        Self {
            version: 1,
            timestamp: Utc::now(),
            mission_id: layout.mission_id.clone(),
            command,
            kind,
            result,
            duration_ms: duration.as_millis().try_into().unwrap_or(u64::MAX),
            metadata,
        }
    }

    pub fn mission_initialized(layout: &MissionLayout, duration: Duration) -> Self {
        Self::new(
            layout,
            "init",
            EventKind::MissionInitialized,
            EventResult::Success,
            duration,
            json!({}),
        )
    }

    pub fn artifact_written(
        layout: &MissionLayout,
        kind: ArtifactKind,
        template_version: u32,
        overwrite: bool,
        path: &Path,
        duration: Duration,
    ) -> Result<Self> {
        Ok(Self::new(
            layout,
            "interview",
            EventKind::ArtifactWritten,
            EventResult::Success,
            duration,
            json!({
                "artifact_kind": kind,
                "template_version": template_version,
                "overwrite": overwrite,
                "path": mission_relative_path(layout, path)?,
            }),
        ))
    }

    pub fn artifact_write_failed(
        layout: &MissionLayout,
        kind: ArtifactKind,
        template_version: u32,
        overwrite: bool,
        error_code: &'static str,
        duration: Duration,
    ) -> Self {
        Self::new(
            layout,
            "interview",
            EventKind::ArtifactWriteFailed,
            EventResult::Error,
            duration,
            json!({
                "artifact_kind": kind,
                "template_version": template_version,
                "overwrite": overwrite,
                "error_code": error_code,
            }),
        )
    }

    pub fn subplan_moved(
        layout: &MissionLayout,
        from_path: &Path,
        to_path: &Path,
        from_lifecycle: SubplanState,
        to_lifecycle: SubplanState,
        duration: Duration,
    ) -> Result<Self> {
        Ok(Self::new(
            layout,
            "subplan move",
            EventKind::SubplanMoved,
            EventResult::Success,
            duration,
            json!({
                "from_path": mission_relative_path(layout, from_path)?,
                "to_path": mission_relative_path(layout, to_path)?,
                "from_lifecycle": from_lifecycle,
                "to_lifecycle": to_lifecycle,
            }),
        ))
    }

    pub fn subplan_move_failed(
        layout: &MissionLayout,
        to_lifecycle: SubplanState,
        error_code: &'static str,
        duration: Duration,
    ) -> Self {
        Self::new(
            layout,
            "subplan move",
            EventKind::SubplanMoveFailed,
            EventResult::Error,
            duration,
            json!({
                "to_lifecycle": to_lifecycle,
                "error_code": error_code,
            }),
        )
    }

    pub fn receipt_appended(
        layout: &MissionLayout,
        path: &Path,
        duration: Duration,
    ) -> Result<Self> {
        Ok(Self::new(
            layout,
            "receipt append",
            EventKind::ReceiptAppended,
            EventResult::Success,
            duration,
            json!({
                "path": mission_relative_path(layout, path)?,
            }),
        ))
    }

    pub fn receipt_append_failed(
        layout: &MissionLayout,
        error_code: &'static str,
        duration: Duration,
    ) -> Self {
        Self::new(
            layout,
            "receipt append",
            EventKind::ReceiptAppendFailed,
            EventResult::Error,
            duration,
            json!({
                "error_code": error_code,
            }),
        )
    }

    pub fn loop_started(
        layout: &MissionLayout,
        mode: &str,
        message_present: bool,
        duration: Duration,
    ) -> Self {
        Self::new(
            layout,
            "loop start",
            EventKind::LoopStarted,
            EventResult::Success,
            duration,
            json!({
                "mode": mode,
                "message_present": message_present,
            }),
        )
    }

    pub fn loop_start_failed(
        layout: &MissionLayout,
        mode: &str,
        message_present: bool,
        error_code: &'static str,
        duration: Duration,
    ) -> Self {
        Self::new(
            layout,
            "loop start",
            EventKind::LoopStartFailed,
            EventResult::Error,
            duration,
            json!({
                "mode": mode,
                "message_present": message_present,
                "error_code": error_code,
            }),
        )
    }

    pub fn loop_paused(layout: &MissionLayout, reason_present: bool, duration: Duration) -> Self {
        Self::new(
            layout,
            "loop pause",
            EventKind::LoopPaused,
            EventResult::Success,
            duration,
            json!({
                "reason_present": reason_present,
            }),
        )
    }

    pub fn loop_pause_failed(
        layout: &MissionLayout,
        reason_present: bool,
        error_code: &'static str,
        duration: Duration,
    ) -> Self {
        Self::new(
            layout,
            "loop pause",
            EventKind::LoopPauseFailed,
            EventResult::Error,
            duration,
            json!({
                "reason_present": reason_present,
                "error_code": error_code,
            }),
        )
    }

    pub fn loop_resumed(layout: &MissionLayout, duration: Duration) -> Self {
        Self::new(
            layout,
            "loop resume",
            EventKind::LoopResumed,
            EventResult::Success,
            duration,
            json!({}),
        )
    }

    pub fn loop_resume_failed(
        layout: &MissionLayout,
        error_code: &'static str,
        duration: Duration,
    ) -> Self {
        Self::new(
            layout,
            "loop resume",
            EventKind::LoopResumeFailed,
            EventResult::Error,
            duration,
            json!({
                "error_code": error_code,
            }),
        )
    }

    pub fn loop_stopped(layout: &MissionLayout, reason_present: bool, duration: Duration) -> Self {
        Self::new(
            layout,
            "loop stop",
            EventKind::LoopStopped,
            EventResult::Success,
            duration,
            json!({
                "reason_present": reason_present,
            }),
        )
    }

    pub fn loop_stop_failed(
        layout: &MissionLayout,
        reason_present: bool,
        error_code: &'static str,
        duration: Duration,
    ) -> Self {
        Self::new(
            layout,
            "loop stop",
            EventKind::LoopStopFailed,
            EventResult::Error,
            duration,
            json!({
                "reason_present": reason_present,
                "error_code": error_code,
            }),
        )
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct EventWarning {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug)]
pub struct EventLogScan {
    pub event_count: usize,
    pub warnings: Vec<EventLogScanWarning>,
}

#[derive(Debug)]
pub struct EventLogScanWarning {
    pub code: &'static str,
    pub detail: String,
}

impl EventWarning {
    fn append_failed(error: Codex1Error) -> Self {
        Self {
            code: "EVENT_LOG_APPEND_FAILED",
            message: format!("event log append failed: {error}"),
        }
    }
}

pub fn append_best_effort(layout: &MissionLayout, record: &EventRecord) -> Option<EventWarning> {
    append(layout, record)
        .err()
        .map(EventWarning::append_failed)
}

pub fn scan(layout: &MissionLayout) -> Result<EventLogScan> {
    let path = layout.event_log();
    let mut warnings = Vec::new();
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            warnings.push(EventLogScanWarning {
                code: "SYMLINKED_PATH",
                detail: mission_relative_path(layout, &path)
                    .unwrap_or_else(|_| path.display().to_string()),
            });
            return Ok(EventLogScan {
                event_count: 0,
                warnings,
            });
        }
        Ok(metadata) if metadata.is_file() => {
            if let Err(error) = ensure_existing_contained(&layout.mission_dir, &path) {
                return handle_unsafe_scan_path(layout, &path, error, warnings);
            }
        }
        Ok(_) => {
            warnings.push(EventLogScanWarning {
                code: "EVENT_LOG_NOT_FILE",
                detail: mission_relative_path(layout, &path)
                    .unwrap_or_else(|_| path.display().to_string()),
            });
            return Ok(EventLogScan {
                event_count: 0,
                warnings,
            });
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(EventLogScan {
                event_count: 0,
                warnings,
            });
        }
        Err(error) => {
            return Err(Codex1Error::Io {
                context: format!("failed to inspect {}", path.display()),
                source: error,
            });
        }
    }

    let file = fs::File::open(&path).io_context(format!("failed to open {}", path.display()))?;
    let mut event_count = 0;
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line_number = index + 1;
        let line = line.io_context(format!("failed to read {}", path.display()))?;
        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => {
                warnings.push(EventLogScanWarning {
                    code: "MALFORMED_EVENT_LOG_LINE",
                    detail: format!("events.jsonl line {line_number}"),
                });
                continue;
            }
        };
        let Some(object) = value.as_object() else {
            warnings.push(EventLogScanWarning {
                code: "NON_OBJECT_EVENT_LOG_LINE",
                detail: format!("events.jsonl line {line_number}"),
            });
            continue;
        };
        if object.get("version").and_then(Value::as_u64) != Some(1) {
            warnings.push(EventLogScanWarning {
                code: "UNSUPPORTED_EVENT_LOG_VERSION",
                detail: format!("events.jsonl line {line_number}"),
            });
            continue;
        }
        if object.get("kind").and_then(Value::as_str).is_none() {
            warnings.push(EventLogScanWarning {
                code: "MISSING_EVENT_LOG_KIND",
                detail: format!("events.jsonl line {line_number}"),
            });
            continue;
        }
        event_count += 1;
    }

    Ok(EventLogScan {
        event_count,
        warnings,
    })
}

fn append(layout: &MissionLayout, record: &EventRecord) -> Result<()> {
    create_dir_all_contained(&layout.mission_dir, ".codex1")?;
    let path = layout.event_log();
    ensure_contained_for_write(&layout.mission_dir, &path)?;
    ensure_append_target_is_regular_file(&path)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .io_context(format!("failed to open {}", path.display()))?;
    let line = serde_json::to_string(record).map_err(|source| Codex1Error::Io {
        context: "failed to serialize event record".into(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, source),
    })?;
    writeln!(file, "{line}").io_context(format!("failed to append {}", path.display()))
}

fn handle_unsafe_scan_path(
    layout: &MissionLayout,
    path: &Path,
    error: Codex1Error,
    mut warnings: Vec<EventLogScanWarning>,
) -> Result<EventLogScan> {
    match error {
        Codex1Error::MissionPath(_) => {
            warnings.push(EventLogScanWarning {
                code: "SYMLINKED_PATH",
                detail: mission_relative_path(layout, path)
                    .unwrap_or_else(|_| path.display().to_string()),
            });
            Ok(EventLogScan {
                event_count: 0,
                warnings,
            })
        }
        error => Err(error),
    }
}

fn ensure_append_target_is_regular_file(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(metadata) if metadata.file_type().is_symlink() => Err(Codex1Error::MissionPath(
            format!("event log target must not be a symlink: {}", path.display()),
        )),
        Ok(_) => Err(Codex1Error::MissionPath(format!(
            "event log target must be a regular file: {}",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Codex1Error::Io {
            context: format!("failed to inspect {}", path.display()),
            source: error,
        }),
    }
}

pub fn mission_relative_path(layout: &MissionLayout, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(&layout.mission_dir).map_err(|_| {
        Codex1Error::MissionPath(format!(
            "event path escapes mission directory: {}",
            path.display()
        ))
    })?;
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(Codex1Error::MissionPath(format!(
            "unsafe event relative path: {}",
            path.display()
        )));
    }
    Ok(relative.display().to_string())
}
