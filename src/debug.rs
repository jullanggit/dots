use crate::{
    DebugCommands::{self, *},
    util::{config_path, system_path},
};

pub fn debug(debug_command: DebugCommands) {
    match debug_command {
        ConfigPath { path } => {
            println!("{}", config_path(&path).display());
        }
        SystemPath { path } => {
            println!("{}", system_path(&path).display());
        }
    }
}
