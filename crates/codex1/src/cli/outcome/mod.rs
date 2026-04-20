//! `codex1 outcome` — OUTCOME.md validation and ratification.
//!
//! `outcome check` is a read-only validator that reports every missing
//! required field and every `[codex1-fill:…]` / boilerplate placeholder.
//!
//! `outcome ratify` re-runs the same validation, then (on success)
//! mutates STATE.json via `state::mutate` to set
//! `outcome.ratified = true`, records `outcome.ratified_at`, advances
//! `phase` from `Clarify` to `Plan`, and flips
//! `status: draft` → `status: ratified` inside OUTCOME.md itself.

use clap::Subcommand;

use crate::cli::Ctx;
use crate::core::error::CliResult;

mod check;
mod emit;
mod ratify;
mod validate;

#[derive(Debug, Subcommand)]
pub enum OutcomeCmd {
    /// Validate OUTCOME.md mechanical completeness.
    Check,
    /// Ratify OUTCOME.md (only if check passes).
    Ratify,
}

pub fn dispatch(cmd: OutcomeCmd, ctx: &Ctx) -> CliResult<()> {
    match cmd {
        OutcomeCmd::Check => check::run(ctx),
        OutcomeCmd::Ratify => ratify::run(ctx),
    }
}
