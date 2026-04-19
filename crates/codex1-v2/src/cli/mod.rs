//! clap-based CLI dispatch.
//!
//! Parses arguments into a [`Cli`] and dispatches to per-command modules.
//! Global flags (`--json`, `--repo-root`) are parsed here and passed through
//! to every command. Repo-root resolution policy: `--repo-root <path>`
//! explicit or cwd; **no git-walk-up**.

// The per-command modules fill in their bodies in T11 (init, validate) and
// T12 (status, plan, task). T10's job is the dispatch surface.
#![allow(dead_code)]

pub(crate) mod init;
pub(crate) mod mission_close;
pub(crate) mod parent_loop;
pub(crate) mod plan;
pub(crate) mod replan;
pub(crate) mod review;
pub(crate) mod status;
pub(crate) mod task;
pub(crate) mod validate;

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use serde_json::Value;

use crate::envelope;
use crate::error::CliError;
use crate::mission::resolve_repo_root;

/// Top-level CLI.
#[derive(Debug, Parser)]
#[command(
    name = "codex1",
    version,
    about = "Codex1 — skills-first harness + CLI contract kernel"
)]
pub struct Cli {
    /// Emit JSON on stdout. Diagnostics still go to stderr.
    #[arg(long, global = true)]
    pub json: bool,

    /// Explicit repo root. Defaults to cwd. No git-walk-up.
    #[arg(long, global = true, value_name = "PATH")]
    pub repo_root: Option<PathBuf>,

    /// Expected `state_revision` before a mutating command. Mismatch →
    /// `REVISION_CONFLICT` (exit 4, retryable). Ignored by non-mutating
    /// commands.
    #[arg(long, global = true, value_name = "N")]
    pub expect_revision: Option<u64>,

    /// Validate preconditions and compute what would change, but do not
    /// write `STATE.json`, do not append to `events.jsonl`, and do not
    /// create review-bundle files. The envelope carries `dry_run: true`
    /// so callers can distinguish a preview from a committed mutation.
    /// Ignored by non-mutating commands.
    #[arg(long, global = true)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level subcommands (Wave 1 surface).
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialise a new mission directory under PLANS/<mission>/.
    Init {
        #[arg(long, value_name = "ID")]
        mission: String,
        #[arg(long, value_name = "TITLE")]
        title: String,
    },
    /// Superset validation: lock + blueprint + DAG + state consistency.
    Validate {
        #[arg(long, value_name = "ID")]
        mission: String,
    },
    /// Emit the status envelope (Ralph consumes this one command only).
    Status {
        #[arg(long, value_name = "ID")]
        mission: String,
    },
    /// Plan subcommands.
    #[command(subcommand)]
    Plan(PlanCommand),
    /// Task subcommands.
    #[command(subcommand)]
    Task(TaskCommand),
    /// Review subcommands.
    #[command(subcommand)]
    Review(ReviewCommand),
    /// Replan subcommands.
    #[command(subcommand)]
    Replan(ReplanCommand),
    /// Parent-loop subcommands: activate, deactivate, pause, resume.
    #[command(name = "parent-loop", subcommand)]
    ParentLoop(ParentLoopCommand),
    /// Mission-close subcommands: readiness check and terminal complete.
    #[command(name = "mission-close", subcommand)]
    MissionClose(MissionCloseCommand),
}

/// `codex1 mission-close <subcommand>`.
#[derive(Debug, Subcommand)]
pub enum MissionCloseCommand {
    /// Readiness report. Refuses to pass until all required conditions hold.
    Check {
        #[arg(long, value_name = "ID")]
        mission: String,
    },
    /// Terminal transition. Refuses unless `check` passes.
    Complete {
        #[arg(long, value_name = "ID")]
        mission: String,
    },
}

/// `codex1 parent-loop <subcommand>`.
#[derive(Debug, Subcommand)]
pub enum ParentLoopCommand {
    /// Mark a parent loop as active with the given mode. Ralph then blocks
    /// stop until `pause` or `deactivate` is invoked.
    Activate {
        #[arg(long, value_name = "ID")]
        mission: String,
        /// One of `execute | review | autopilot | close`.
        #[arg(long, value_name = "MODE")]
        mode: String,
    },
    /// Mark the parent loop as inactive (mode reset to `none`).
    Deactivate {
        #[arg(long, value_name = "ID")]
        mission: String,
    },
    /// Pause the active loop — Ralph will then allow stop with reason
    /// `discussion_pause`.
    Pause {
        #[arg(long, value_name = "ID")]
        mission: String,
    },
    /// Resume a paused parent loop.
    Resume {
        #[arg(long, value_name = "ID")]
        mission: String,
    },
}

/// `codex1 review <subcommand>`.
#[derive(Debug, Subcommand)]
pub enum ReviewCommand {
    /// Open a review bundle for a task with the listed profiles.
    Open {
        #[arg(long, value_name = "ID")]
        mission: String,
        #[arg(long, value_name = "TASK")]
        task: String,
        /// Comma-separated profile list (e.g. `code_bug_correctness,local_spec_intent`).
        #[arg(long, value_name = "PROFILES")]
        profiles: String,
    },
    /// Open a mission-close review bundle (no task scope).
    #[command(name = "open-mission-close")]
    OpenMissionClose {
        #[arg(long, value_name = "ID")]
        mission: String,
        #[arg(long, value_name = "PROFILES")]
        profiles: String,
    },
    /// Submit a reviewer output to a bundle.
    Submit {
        #[arg(long, value_name = "ID")]
        mission: String,
        #[arg(long, value_name = "BUNDLE")]
        bundle: String,
        /// JSON file produced by the reviewer (path relative to repo root).
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
    },
    /// Read the cleanliness status of a bundle.
    Status {
        #[arg(long, value_name = "ID")]
        mission: String,
        #[arg(long, value_name = "BUNDLE")]
        bundle: String,
    },
    /// Close a bundle — transitions the task `review_owed` →
    /// `review_clean` (clean) or `review_failed` (blocking findings).
    Close {
        #[arg(long, value_name = "ID")]
        mission: String,
        #[arg(long, value_name = "BUNDLE")]
        bundle: String,
    },
}

/// `codex1 replan <subcommand>`.
#[derive(Debug, Subcommand)]
pub enum ReplanCommand {
    /// Record a replan event with a reason code.
    Record {
        #[arg(long, value_name = "ID")]
        mission: String,
        #[arg(long, value_name = "CODE")]
        reason: String,
        /// Comma-separated task ids this replan supersedes (optional).
        #[arg(long, value_name = "IDS")]
        supersedes: Option<String>,
    },
    /// Check for mandatory replan triggers.
    Check {
        #[arg(long, value_name = "ID")]
        mission: String,
    },
}

/// `codex1 plan <subcommand>`.
#[derive(Debug, Subcommand)]
pub enum PlanCommand {
    /// DAG-only validation (ID format, cycles, deps, duplicates, schema).
    Check {
        #[arg(long, value_name = "ID")]
        mission: String,
    },
    /// Emit the parsed DAG as JSON.
    Graph {
        #[arg(long, value_name = "ID")]
        mission: String,
    },
    /// Derive wave schedule from DAG + STATE.
    Waves {
        #[arg(long, value_name = "ID")]
        mission: String,
    },
}

/// `codex1 task <subcommand>`.
#[derive(Debug, Subcommand)]
pub enum TaskCommand {
    /// First eligible task per wave derivation (lowest id).
    Next {
        #[arg(long, value_name = "ID")]
        mission: String,
    },
    /// Transition a `Planned` task → `Ready`. Driven by `$plan` after writing
    /// the spec file. Goes through `StateStore::mutate` so `state_revision`
    /// bumps and a `task_marked_ready` event lands in `events.jsonl`.
    Ready {
        #[arg(long, value_name = "ID")]
        mission: String,
        #[arg(value_name = "TASK")]
        task_id: String,
    },
    /// Transition a `Ready`/`NeedsRepair` task → `InProgress`; mint a `task_run_id`.
    Start {
        #[arg(long, value_name = "ID")]
        mission: String,
        #[arg(value_name = "TASK")]
        task_id: String,
    },
    /// Transition `InProgress` → `ProofSubmitted`; record proof hash.
    Finish {
        #[arg(long, value_name = "ID")]
        mission: String,
        #[arg(value_name = "TASK")]
        task_id: String,
        /// Path (relative to mission dir) of the proof file. Defaults to
        /// `specs/T<N>/PROOF.md`.
        #[arg(long, value_name = "PATH")]
        proof: Option<PathBuf>,
    },
    /// Read a single task's state.
    Status {
        #[arg(long, value_name = "ID")]
        mission: String,
        #[arg(value_name = "TASK")]
        task_id: String,
    },
}

/// Run the parsed CLI and return the process exit code.
#[must_use]
pub fn run(cli: &Cli) -> i32 {
    match &cli.command {
        Commands::Init { mission, title } => init::cmd_init(cli, mission, title),
        Commands::Validate { mission } => validate::cmd_validate(cli, mission),
        Commands::Status { mission } => status::cmd_status(cli, mission),
        Commands::Plan(PlanCommand::Check { mission }) => plan::cmd_plan_check(cli, mission),
        Commands::Plan(PlanCommand::Graph { mission }) => plan::cmd_plan_graph(cli, mission),
        Commands::Plan(PlanCommand::Waves { mission }) => plan::cmd_plan_waves(cli, mission),
        Commands::Task(TaskCommand::Next { mission }) => task::cmd_task_next(cli, mission),
        Commands::Task(TaskCommand::Ready { mission, task_id }) => {
            task::cmd_task_ready(cli, mission, task_id)
        }
        Commands::Task(TaskCommand::Start { mission, task_id }) => {
            task::cmd_task_start(cli, mission, task_id)
        }
        Commands::Task(TaskCommand::Finish {
            mission,
            task_id,
            proof,
        }) => task::cmd_task_finish(cli, mission, task_id, proof.as_deref()),
        Commands::Task(TaskCommand::Status { mission, task_id }) => {
            task::cmd_task_status(cli, mission, task_id)
        }
        Commands::Review(ReviewCommand::Open {
            mission,
            task,
            profiles,
        }) => review::cmd_review_open(cli, mission, task, profiles),
        Commands::Review(ReviewCommand::OpenMissionClose { mission, profiles }) => {
            review::cmd_review_open_mission_close(cli, mission, profiles)
        }
        Commands::Review(ReviewCommand::Submit {
            mission,
            bundle,
            input,
        }) => review::cmd_review_submit(cli, mission, bundle, input),
        Commands::Review(ReviewCommand::Status { mission, bundle }) => {
            review::cmd_review_status(cli, mission, bundle)
        }
        Commands::Review(ReviewCommand::Close { mission, bundle }) => {
            review::cmd_review_close(cli, mission, bundle)
        }
        Commands::Replan(ReplanCommand::Record {
            mission,
            reason,
            supersedes,
        }) => replan::cmd_replan_record(cli, mission, reason, supersedes.as_deref()),
        Commands::Replan(ReplanCommand::Check { mission }) => {
            replan::cmd_replan_check(cli, mission)
        }
        Commands::ParentLoop(ParentLoopCommand::Activate { mission, mode }) => {
            parent_loop::cmd_activate(cli, mission, mode)
        }
        Commands::ParentLoop(ParentLoopCommand::Deactivate { mission }) => {
            parent_loop::cmd_deactivate(cli, mission)
        }
        Commands::ParentLoop(ParentLoopCommand::Pause { mission }) => {
            parent_loop::cmd_pause(cli, mission)
        }
        Commands::ParentLoop(ParentLoopCommand::Resume { mission }) => {
            parent_loop::cmd_resume(cli, mission)
        }
        Commands::MissionClose(MissionCloseCommand::Check { mission }) => {
            mission_close::cmd_mission_close_check(cli, mission)
        }
        Commands::MissionClose(MissionCloseCommand::Complete { mission }) => {
            mission_close::cmd_mission_close_complete(cli, mission)
        }
    }
}

/// Resolve the repo root from the global `--repo-root` or cwd.
pub fn resolve_repo(cli: &Cli) -> Result<PathBuf, CliError> {
    resolve_repo_root(cli.repo_root.as_deref())
}

/// Emit a successful command result.
///
/// If `cli.json` is set, writes the envelope JSON to stdout on one line.
/// Otherwise writes a short human summary taken from the `message` key (if
/// present) or the full JSON as a fallback.
///
/// When `cli.dry_run` is true the function injects `"dry_run": true` into
/// the envelope so callers can mechanically distinguish a preview from a
/// committed mutation — added in Round 6 Fix #5 rather than threading the
/// flag through every command's `json!(...)` block.
#[must_use]
pub fn emit_success(cli: &Cli, envelope: &Value) -> i32 {
    let envelope_with_flag = if cli.dry_run {
        let mut v = envelope.clone();
        if let Some(obj) = v.as_object_mut() {
            obj.insert("dry_run".into(), Value::Bool(true));
        }
        v
    } else {
        envelope.clone()
    };
    let emit = &envelope_with_flag;
    if cli.json {
        writeln_stdout(&envelope::to_string(emit));
    } else if let Some(msg) = emit.get("message").and_then(Value::as_str) {
        let prefix = if cli.dry_run { "[dry-run] " } else { "" };
        writeln_stdout(&format!("{prefix}{msg}"));
    } else {
        writeln_stdout(&envelope::to_string(emit));
    }
    0
}

/// Emit an error and return its exit code.
#[must_use]
pub fn emit_error(cli: &Cli, err: &CliError) -> i32 {
    let env = envelope::error(err);
    if cli.json {
        writeln_stdout(&envelope::to_string(&env));
    } else {
        let msg = format!("error [{}]: {}", err.code(), err);
        writeln_stderr(&msg);
        if let Some(hint) = err.hint() {
            writeln_stderr(&format!("hint: {hint}"));
        }
    }
    err.exit_code()
}

/// Current time as RFC-3339 UTC.
#[must_use]
pub(crate) fn now_rfc3339() -> String {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn writeln_stdout(s: &str) {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(handle, "{s}");
}

fn writeln_stderr(s: &str) {
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    let _ = writeln!(handle, "{s}");
}

/// Helper used by per-command stubs until T11/T12 land their bodies.
pub(crate) fn not_implemented(cli: &Cli, command: &str) -> i32 {
    let err = CliError::Internal {
        message: format!("command {command:?} not yet implemented in this wave"),
    };
    emit_error(cli, &err)
}

/// Build a Cli from an args slice (useful in tests and in main).
#[must_use]
pub fn parse_args<I, T>(args: I) -> Cli
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    Cli::parse_from(args)
}

/// Return the `mission_dir` path under the resolved repo root, without
/// creating it. Used by non-mutating commands.
pub fn mission_dir(repo_root: &Path, mission_id: &str) -> Result<PathBuf, CliError> {
    let paths = crate::mission::resolve_mission(repo_root, mission_id)?;
    Ok(paths.mission_dir)
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands, PlanCommand, TaskCommand};
    use clap::Parser;

    #[test]
    fn top_level_help_lists_subcommands() {
        let out = Cli::try_parse_from(["codex1-v2", "--help"]).unwrap_err();
        let s = out.to_string();
        for cmd in ["init", "validate", "status", "plan", "task"] {
            assert!(s.contains(cmd), "help should list {cmd}: {s}");
        }
    }

    #[test]
    fn init_requires_mission_and_title() {
        let r = Cli::try_parse_from(["codex1-v2", "init"]);
        assert!(r.is_err());
        let r = Cli::try_parse_from(["codex1-v2", "init", "--mission", "abc"]);
        assert!(r.is_err());
        let r = Cli::try_parse_from(["codex1-v2", "init", "--mission", "abc", "--title", "Foo"]);
        assert!(r.is_ok());
    }

    #[test]
    fn global_flags_work_on_any_subcommand() {
        let cli =
            Cli::try_parse_from(["codex1-v2", "--json", "status", "--mission", "m1"]).unwrap();
        assert!(cli.json);
        match cli.command {
            Commands::Status { mission } => assert_eq!(mission, "m1"),
            other => panic!("expected Status, got {other:?}"),
        }
    }

    #[test]
    fn repo_root_accepted() {
        let cli = Cli::try_parse_from([
            "codex1-v2",
            "--repo-root",
            "/tmp/repo",
            "status",
            "--mission",
            "m1",
        ])
        .unwrap();
        assert_eq!(
            cli.repo_root.as_ref().unwrap().display().to_string(),
            "/tmp/repo"
        );
    }

    #[test]
    fn plan_check_parses() {
        let cli = Cli::try_parse_from(["codex1-v2", "plan", "check", "--mission", "m1"]).unwrap();
        match cli.command {
            Commands::Plan(PlanCommand::Check { mission }) => {
                assert_eq!(mission, "m1");
            }
            other => panic!("expected plan check, got {other:?}"),
        }
    }

    #[test]
    fn plan_waves_parses() {
        let cli = Cli::try_parse_from(["codex1-v2", "plan", "waves", "--mission", "m1"]).unwrap();
        match cli.command {
            Commands::Plan(PlanCommand::Waves { mission }) => {
                assert_eq!(mission, "m1");
            }
            other => panic!("expected plan waves, got {other:?}"),
        }
    }

    #[test]
    fn task_next_parses() {
        let cli = Cli::try_parse_from(["codex1-v2", "task", "next", "--mission", "m1"]).unwrap();
        match cli.command {
            Commands::Task(TaskCommand::Next { mission }) => {
                assert_eq!(mission, "m1");
            }
            other => panic!("expected task next, got {other:?}"),
        }
    }

    #[test]
    fn unknown_subcommand_rejected() {
        let r = Cli::try_parse_from(["codex1-v2", "frobnicate"]);
        assert!(r.is_err());
    }

    #[test]
    fn validate_requires_mission() {
        assert!(Cli::try_parse_from(["codex1-v2", "validate"]).is_err());
        assert!(Cli::try_parse_from(["codex1-v2", "validate", "--mission", "m1",]).is_ok());
    }
}
