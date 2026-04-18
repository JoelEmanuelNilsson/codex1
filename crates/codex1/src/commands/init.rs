use anyhow::Result;

use crate::commands::{InitArgs, SetupArgs};

pub fn run(args: InitArgs) -> Result<()> {
    crate::commands::setup::run_project_setup(SetupArgs {
        common: args.common,
        backup_root: args.backup_root,
        force: args.force,
    })
}
