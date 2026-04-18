//! Codex1 V2 CLI entry point — thin dispatcher over `codex1_v2::cli::run`.

use clap::Parser;
use codex1_v2::cli::{Cli, run};

fn main() {
    let cli = Cli::parse();
    std::process::exit(run(&cli));
}
