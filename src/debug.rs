use color_eyre::eyre::Result;

use crate::{
    DebugCommands::{self, ConfigPath, SystemPath},
    util::{config_path, system_path},
};

pub fn debug(debug_command: DebugCommands) -> Result<()> {
    match debug_command {
        ConfigPath { path } => {
            println!("{}", config_path(&path)?.display());
        }
        SystemPath { path } => {
            println!("{}", system_path(&path)?.display());
        }
    }

    Ok(())
}
