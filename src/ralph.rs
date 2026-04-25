use std::io::{self, Read};
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::layout::MissionLayout;
use crate::loop_state;
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
    let layout = if let Some(id) = mission {
        match MissionLayout::new(root, id) {
            Ok(layout) => layout,
            Err(_) => return StopHookOutput::approve(),
        }
    } else {
        match MissionLayout::from_cwd(root, &cwd) {
            Some(layout) => layout,
            None => return StopHookOutput::approve(),
        }
    };
    let Some(state) = loop_state::read_optional(&layout) else {
        return StopHookOutput::approve();
    };
    if !state.active || state.paused || state.message.trim().is_empty() {
        return StopHookOutput::approve();
    }
    StopHookOutput::block(format!(
        "{}\n\nPause or stop this loop with: {}",
        state.message, state.pause_command
    ))
}
