mod cli;
mod command;
mod envelope;
mod error;
mod layout;
mod paths;
mod setup;

use std::process::ExitCode;

fn main() -> ExitCode {
    command::run()
}
