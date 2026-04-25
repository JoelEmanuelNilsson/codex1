mod cli;
mod command;
mod envelope;
mod error;
mod inspect;
mod interview;
mod layout;
mod loop_state;
mod paths;
mod ralph;
mod render;
mod template;

use std::process::ExitCode;

fn main() -> ExitCode {
    command::run()
}
