use std::fs;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Codex1Error, IoContext, Result};
use crate::layout::MissionLayout;
use crate::paths::{
    create_dir_all_contained, ensure_contained_for_write, ensure_existing_contained,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoopState {
    pub version: u32,
    pub active: bool,
    pub paused: bool,
    pub mode: String,
    pub message: String,
    pub pause_command: String,
    #[serde(default)]
    pub stop_command: String,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pause_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}

impl LoopState {
    pub fn start(mode: String, message: String, layout: &MissionLayout) -> Result<Self> {
        if message.trim().is_empty() {
            return Err(Codex1Error::Loop("loop message must not be empty".into()));
        }
        Ok(Self {
            version: 1,
            active: true,
            paused: false,
            mode,
            message,
            pause_command: pause_command(layout),
            stop_command: stop_command(layout),
            updated_at: Utc::now(),
            pause_reason: None,
            stop_reason: None,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            return Err(Codex1Error::Loop(format!(
                "unsupported loop state version {}",
                self.version
            )));
        }
        Ok(())
    }
}

pub fn read(layout: &MissionLayout) -> Result<LoopState> {
    let path = layout.loop_file();
    ensure_existing_contained(&layout.mission_dir, &path)?;
    let text =
        fs::read_to_string(&path).io_context(format!("failed to read {}", path.display()))?;
    let mut state: LoopState = serde_json::from_str(&text)
        .map_err(|source| Codex1Error::Loop(format!("failed to parse loop state: {source}")))?;
    state.validate()?;
    if state.pause_command.trim().is_empty() {
        state.pause_command = pause_command(layout);
    }
    if state.stop_command.trim().is_empty() {
        state.stop_command = stop_command(layout);
    }
    Ok(state)
}

pub fn pause_command(layout: &MissionLayout) -> String {
    format!(
        "codex1 --mission={} loop pause --reason <reason>",
        layout.mission_id
    )
}

pub fn stop_command(layout: &MissionLayout) -> String {
    format!(
        "codex1 --mission={} loop stop --reason <reason>",
        layout.mission_id
    )
}

pub fn read_optional(layout: &MissionLayout) -> Option<LoopState> {
    read(layout).ok()
}

pub fn write(layout: &MissionLayout, state: &LoopState) -> Result<()> {
    create_dir_all_contained(&layout.mission_dir, ".codex1")?;
    let text = serde_json::to_string_pretty(state)
        .map_err(|source| Codex1Error::Loop(format!("failed to serialize loop state: {source}")))?;
    ensure_contained_for_write(&layout.mission_dir, &layout.loop_file())?;
    fs::write(layout.loop_file(), format!("{text}\n"))
        .io_context(format!("failed to write {}", layout.loop_file().display()))
}

pub fn pause(layout: &MissionLayout, reason: Option<String>) -> Result<LoopState> {
    let mut state = read(layout)?;
    state.paused = true;
    state.pause_reason = reason;
    state.updated_at = Utc::now();
    write(layout, &state)?;
    Ok(state)
}

pub fn resume(layout: &MissionLayout) -> Result<LoopState> {
    let mut state = read(layout)?;
    state.paused = false;
    state.pause_reason = None;
    state.updated_at = Utc::now();
    write(layout, &state)?;
    Ok(state)
}

pub fn stop(layout: &MissionLayout, reason: Option<String>) -> Result<LoopState> {
    let mut state = read(layout)?;
    state.active = false;
    state.paused = false;
    state.stop_reason = reason;
    state.updated_at = Utc::now();
    write(layout, &state)?;
    Ok(state)
}
