//! Clap dispatch tree.
//!
//! Foundation pins the full subcommand structure here so Phase B workers
//! can implement their handlers without touching this file. Each feature
//! sub-module exposes `dispatch(cmd, &Ctx) -> CliResult<()>`; adding a
//! subvariant requires a coordinated Foundation change.
//!
//! Global flags (`--mission`, `--repo-root`, `--json`, `--dry-run`,
//! `--expect-revision`) are available on every subcommand.

pub mod close;
pub mod doctor;
pub mod hook;
pub mod init;
pub mod loop_;
pub mod outcome;
pub mod plan;
pub mod replan;
pub mod review;
pub mod status;
pub mod task;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::core::error::{CliError, CliResult};
use crate::core::mission::MissionSelector;

/// Convenience alias so every dispatcher has a consistent context.
pub use Ctx as CliCtx;

/// Global CLI flags shared by every command.
#[derive(Debug, Parser)]
#[command(
    name = "codex1",
    version,
    about = "Skills-first native Codex workflow harness.",
    long_about = "Drives a mission through clarify → plan → execute → review-loop → \
        close. Used by the six public skills ($clarify, $plan, $execute, $review-loop, \
        $close, $autopilot). Ralph stop hooks read only `codex1 status --json`."
)]
pub struct Cli {
    /// Mission id (directory name under PLANS/).
    #[arg(long, global = true, value_name = "ID")]
    pub mission: Option<String>,

    /// Repo root containing PLANS/. Defaults to current directory.
    #[arg(long, global = true, value_name = "PATH")]
    pub repo_root: Option<PathBuf>,

    /// Emit JSON output (currently the default on all commands). Kept for
    /// parity with the cli-creator convention and for forward-compat with
    /// any future `--human` mode.
    #[arg(long, global = true)]
    pub json: bool,

    /// Plan / simulate without mutating STATE.json or appending events.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Enforce strict equality with the current STATE.json revision before
    /// mutating. Returns `REVISION_CONFLICT` on mismatch.
    #[arg(long, global = true, value_name = "N")]
    pub expect_revision: Option<u64>,

    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level subcommand surface. Foundation owns this enum; Phase B
/// workers implement the handler bodies.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create PLANS/<mission>/ with blank OUTCOME.md, PLAN.yaml, STATE.json.
    Init(init::InitArgs),
    /// Report CLI health. Never crashes on missing auth or config.
    Doctor,
    /// Print the one-liner for wiring the Ralph Stop hook.
    Hook {
        #[command(subcommand)]
        cmd: hook::HookCmd,
    },
    /// OUTCOME.md validation and ratification.
    Outcome {
        #[command(subcommand)]
        cmd: outcome::OutcomeCmd,
    },
    /// Plan commands (choose-level, scaffold, check, graph, waves).
    Plan {
        #[command(subcommand)]
        cmd: plan::PlanCmd,
    },
    /// Task lifecycle commands.
    Task {
        #[command(subcommand)]
        cmd: task::TaskCmd,
    },
    /// Review recording and packet emission.
    Review {
        #[command(subcommand)]
        cmd: review::ReviewCmd,
    },
    /// Replan trigger checks and records.
    Replan {
        #[command(subcommand)]
        cmd: replan::ReplanCmd,
    },
    /// Loop pause/resume/deactivate (used by $close).
    Loop {
        #[command(subcommand)]
        cmd: loop_::LoopCmd,
    },
    /// Close check and complete.
    Close {
        #[command(subcommand)]
        cmd: close::CloseCmd,
    },
    /// Unified mission status — the Ralph-facing single source of truth.
    Status,
}

/// Context passed to every command handler.
pub struct Ctx {
    pub mission: Option<String>,
    pub repo_root: Option<PathBuf>,
    pub json: bool,
    pub dry_run: bool,
    pub expect_revision: Option<u64>,
}

impl Ctx {
    #[must_use]
    pub fn selector(&self) -> MissionSelector {
        MissionSelector {
            mission: self.mission.clone(),
            repo_root: self.repo_root.clone(),
        }
    }
}

/// Run the CLI. Prints JSON envelopes to stdout; returns errors for the
/// binary entry point to map into an exit code.
pub fn dispatch() -> CliResult<()> {
    let cli = Cli::parse();
    let ctx = Ctx {
        mission: cli.mission,
        repo_root: cli.repo_root,
        json: cli.json,
        dry_run: cli.dry_run,
        expect_revision: cli.expect_revision,
    };

    let result = match cli.command {
        Commands::Init(args) => init::run(args, &ctx),
        Commands::Doctor => doctor::run(&ctx),
        Commands::Hook { cmd } => hook::dispatch(cmd, &ctx),
        Commands::Outcome { cmd } => outcome::dispatch(cmd, &ctx),
        Commands::Plan { cmd } => plan::dispatch(cmd, &ctx),
        Commands::Task { cmd } => task::dispatch(cmd, &ctx),
        Commands::Review { cmd } => review::dispatch(cmd, &ctx),
        Commands::Replan { cmd } => replan::dispatch(cmd, &ctx),
        Commands::Loop { cmd } => loop_::dispatch(cmd, &ctx),
        Commands::Close { cmd } => close::dispatch(cmd, &ctx),
        Commands::Status => status::run(&ctx),
    };

    match result {
        Ok(()) => Ok(()),
        Err(err) => {
            print_error(&err);
            Err(err)
        }
    }
}

/// Print an error envelope to stdout. Downstream commands should prefer
/// constructing their own `CliError`; only if the error escapes to
/// `dispatch()` do we serialize it here.
fn print_error(err: &CliError) {
    let env = err.to_envelope();
    println!("{}", env.to_pretty());
}
