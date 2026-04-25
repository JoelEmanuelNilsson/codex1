use std::io::{self, Read};
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::layout::MissionLayout;
use crate::loop_state;
use crate::paths::discover_repo_root;

#[derive(Debug, Deserialize)]
struct StopHookInput {
    #[serde(default)]
    stop_hook_active: bool,
    cwd: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum StopHookOutput {
    Approve,
    Block { reason: String },
}

pub fn stop_hook(repo_root: Option<PathBuf>, mission: Option<String>) -> StopHookOutput {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return StopHookOutput::Approve;
    }
    let parsed: StopHookInput = if input.trim().is_empty() {
        StopHookInput {
            stop_hook_active: false,
            cwd: None,
        }
    } else {
        match serde_json::from_str(&input) {
            Ok(value) => value,
            Err(_) => return StopHookOutput::Approve,
        }
    };
    if parsed.stop_hook_active {
        return StopHookOutput::Approve;
    }
    let root = match discover_repo_root(repo_root) {
        Ok(root) => root,
        Err(_) => return StopHookOutput::Approve,
    };
    let layout = if let Some(id) = mission {
        match MissionLayout::new(root, id) {
            Ok(layout) => layout,
            Err(_) => return StopHookOutput::Approve,
        }
    } else {
        let cwd = parsed
            .cwd
            .unwrap_or_else(|| std::env::current_dir().unwrap_or(root.clone()));
        match MissionLayout::from_cwd(root, &cwd) {
            Some(layout) => layout,
            None => return StopHookOutput::Approve,
        }
    };
    let Some(state) = loop_state::read_optional(&layout) else {
        return StopHookOutput::Approve;
    };
    if !state.active || state.paused || state.message.trim().is_empty() {
        return StopHookOutput::Approve;
    }
    StopHookOutput::Block {
        reason: format!(
            "{}\n\nPause or stop this loop with: {}",
            state.message, state.pause_command
        ),
    }
}
