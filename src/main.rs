mod cli;
mod command;
mod envelope;
mod error;
mod event;
mod inspect;
mod interview;
mod layout;
mod paths;
mod render;
mod template;

use std::process::ExitCode;

fn main() -> ExitCode {
    command::run()
}
