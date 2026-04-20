//! Codex1 — skills-first native Codex workflow harness.
//!
//! This crate exposes a deterministic CLI that six public skills
//! (`$clarify`, `$plan`, `$execute`, `$review-loop`, `$close`, `$autopilot`)
//! drive to manage long-running Codex missions.
//!
//! The CLI is the substrate; skills and the main Codex thread own the
//! reasoning. The CLI never inspects caller identity, never stores
//! derived truth (waves), and never hides state in `.ralph/`.

pub mod cli;
pub mod core;
pub mod state;

use std::process::ExitCode;

/// Binary entry point. Runs the clap dispatch and translates the result
/// into a process exit code (`0` = success, `1` = handled error, `2` = bug).
pub fn run() -> ExitCode {
    match cli::dispatch() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // Errors are already printed as JSON/text inside the dispatch;
            // here we only translate to an exit code.
            match err.kind() {
                core::error::ExitKind::HandledError => ExitCode::from(1),
                core::error::ExitKind::Bug => ExitCode::from(2),
            }
        }
    }
}
