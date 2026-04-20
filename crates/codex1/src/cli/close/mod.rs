//! `codex1 close` — mission-close readiness, mission-close review, terminal close.
//!
//! Three subcommands share the same verdict source:
//!
//! - `check` — read-only projection of `state::readiness::derive_verdict`
//!   with a concrete blocker list.
//! - `record-review` — record the mission-close review outcome (clean or
//!   findings-file). Dirty reviews increment the shared replan counter
//!   under the `__mission_close__` target.
//! - `complete` — idempotent terminal-close transition that writes
//!   `CLOSEOUT.md`. Fails closed if `check` is not `ready: true`.
//!
//! `close check` and `status` call the same `readiness::derive_verdict`
//! helper: the two CLI surfaces cannot disagree about whether a mission
//! is ready to close.

use std::path::PathBuf;

use clap::Subcommand;

use crate::cli::Ctx;
use crate::core::error::CliResult;

pub mod check;
pub mod closeout;
pub mod complete;
pub mod record_review;

/// Internal target key for the mission-close review dirty counter.
pub(crate) const MISSION_CLOSE_TARGET: &str = "__mission_close__";

/// Serialize a unit-variant enum (e.g. `TaskStatus`, `ReviewVerdict`,
/// `PlanLevel`) to its canonical snake_case string. Relies on the
/// `serde(rename_all = "snake_case")` pinned by the state schema so the
/// CLI never drifts from the on-disk vocabulary.
pub(crate) fn serde_variant<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(ToString::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

#[derive(Debug, Subcommand)]
pub enum CloseCmd {
    /// Check whether terminal close is ready.
    Check,
    /// Write CLOSEOUT.md and mark the mission terminal.
    Complete,
    /// Record the mission-close review outcome (clean or findings-file).
    RecordReview {
        /// Mark the mission-close review clean (no P0/P1/P2 findings).
        #[arg(long, conflicts_with = "findings_file")]
        clean: bool,
        /// Path to a markdown file describing P0/P1/P2 findings.
        #[arg(long, value_name = "PATH")]
        findings_file: Option<PathBuf>,
        /// Comma-separated reviewer actor ids recorded in the event payload.
        #[arg(long, value_name = "LIST")]
        reviewers: Option<String>,
    },
}

pub fn dispatch(cmd: CloseCmd, ctx: &Ctx) -> CliResult<()> {
    match cmd {
        CloseCmd::Check => check::run(ctx),
        CloseCmd::Complete => complete::run(ctx),
        CloseCmd::RecordReview {
            clean,
            findings_file,
            reviewers,
        } => record_review::run(ctx, clean, findings_file, reviewers.as_deref()),
    }
}
