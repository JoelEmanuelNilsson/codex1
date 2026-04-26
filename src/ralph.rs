use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::layout::MissionLayout;
use crate::loop_state::{self, LoopState};
use crate::paths::{discover_repo_root, discover_repo_root_from};

#[derive(Debug, Deserialize)]
struct StopHookInput {
    #[serde(default)]
    stop_hook_active: bool,
    cwd: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct StopHookOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    decision: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

impl StopHookOutput {
    fn approve() -> Self {
        Self {
            decision: None,
            reason: None,
        }
    }

    fn block(reason: String) -> Self {
        Self {
            decision: Some("block"),
            reason: Some(reason),
        }
    }
}

pub fn stop_hook(repo_root: Option<PathBuf>, mission: Option<String>) -> StopHookOutput {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return StopHookOutput::approve();
    }
    let parsed: StopHookInput = if input.trim().is_empty() {
        StopHookInput {
            stop_hook_active: false,
            cwd: None,
        }
    } else {
        match serde_json::from_str(&input) {
            Ok(value) => value,
            Err(_) => return StopHookOutput::approve(),
        }
    };
    if parsed.stop_hook_active {
        return StopHookOutput::approve();
    }
    let cwd = parsed
        .cwd
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let root = match repo_root {
        Some(root) => discover_repo_root(Some(root)),
        None => discover_repo_root_from(&cwd),
    };
    let root = match root {
        Ok(root) => root,
        Err(_) => return StopHookOutput::approve(),
    };
    let active_loops = if let Some(id) = mission {
        let layout = match MissionLayout::new(root, id) {
            Ok(layout) => layout,
            Err(_) => return StopHookOutput::approve(),
        };
        active_loop_for_layout(layout).into_iter().collect()
    } else {
        match MissionLayout::from_cwd(root.clone(), &cwd) {
            Some(layout) => active_loop_for_layout(layout).into_iter().collect(),
            None => active_loops_under_repo(&root),
        }
    };

    match active_loops.as_slice() {
        [] => StopHookOutput::approve(),
        [(layout, state)] => StopHookOutput::block(block_reason(layout, state)),
        loops => StopHookOutput::block(multiple_block_reason(loops)),
    }
}

fn active_loop_for_layout(layout: MissionLayout) -> Option<(MissionLayout, LoopState)> {
    let state = loop_state::read_optional(&layout)?;
    if !state.active || state.paused || state.message.trim().is_empty() {
        return None;
    }
    Some((layout, state))
}

fn active_loops_under_repo(root: &std::path::Path) -> Vec<(MissionLayout, LoopState)> {
    let missions_dir = root.join(".codex1").join("missions");
    let Ok(metadata) = fs::symlink_metadata(&missions_dir) else {
        return Vec::new();
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(missions_dir) else {
        return Vec::new();
    };
    let mut loops = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        let Some(id) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        let Ok(layout) = MissionLayout::new(root.to_path_buf(), id) else {
            continue;
        };
        if let Some(active) = active_loop_for_layout(layout) {
            loops.push(active);
        }
    }
    loops.sort_by(|(left, _), (right, _)| left.mission_id.cmp(&right.mission_id));
    loops
}

fn block_reason(layout: &MissionLayout, state: &LoopState) -> String {
    let pause = if state.pause_command.trim().is_empty() {
        loop_state::pause_command(layout)
    } else {
        state.pause_command.clone()
    };
    let stop = if state.stop_command.trim().is_empty() {
        loop_state::stop_command(layout)
    } else {
        state.stop_command.clone()
    };
    format!(
        "{}\n\nPause this loop with: {}\nStop this loop with: {}",
        state.message, pause, stop
    )
}

fn multiple_block_reason(loops: &[(MissionLayout, LoopState)]) -> String {
    let mut reason = String::from("Multiple active Codex1 loops exist:\n");
    for (layout, state) in loops {
        reason.push_str(&format!(
            "\n- {}: {}",
            layout.mission_id,
            state.message.trim()
        ));
    }
    reason.push_str("\n\nPause or stop a loop with:");
    for (layout, state) in loops {
        let pause = if state.pause_command.trim().is_empty() {
            loop_state::pause_command(layout)
        } else {
            state.pause_command.clone()
        };
        let stop = if state.stop_command.trim().is_empty() {
            loop_state::stop_command(layout)
        } else {
            state.stop_command.clone()
        };
        reason.push_str(&format!(
            "\n- {}: pause `{}` or stop `{}`",
            layout.mission_id, pause, stop
        ));
    }
    reason
}
