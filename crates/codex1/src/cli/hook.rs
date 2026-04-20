//! `codex1 hook snippet` — prints the one-liner for wiring the Ralph
//! Stop hook. Phase B Unit 12 owns the shell script itself; this command
//! only emits the documented install instructions.

use clap::Subcommand;
use serde_json::json;

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::CliResult;

#[derive(Debug, Subcommand)]
pub enum HookCmd {
    /// Print the one-liner for installing the Ralph Stop hook.
    Snippet,
}

pub fn dispatch(cmd: HookCmd, ctx: &Ctx) -> CliResult<()> {
    match cmd {
        HookCmd::Snippet => snippet(ctx),
    }
}

fn snippet(_ctx: &Ctx) -> CliResult<()> {
    let env = JsonOk::global(json!({
        "hook": {
            "event": "Stop",
            "script_path_hint": "<repo-root>/scripts/ralph-stop-hook.sh",
            "behavior": "Runs `codex1 status --json`; exits 2 to block Stop iff loop.active && !paused && !stop.allow; exits 0 otherwise.",
        },
        "install": {
            "codex_hooks_json_example": {
                "Stop": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "<repo-root>/scripts/ralph-stop-hook.sh"
                            }
                        ]
                    }
                ]
            }
        },
        "note": "The shell script lives at scripts/ralph-stop-hook.sh (Unit 12). This command only prints the wiring instructions.",
    }));
    println!("{}", env.to_pretty());
    Ok(())
}
