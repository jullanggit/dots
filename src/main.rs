#![feature(let_chains)]

mod add;
mod config;
mod debug;
mod import;
mod list;
mod remove;
mod util;

use clap::{Parser, Subcommand};
use std::{path::PathBuf, sync::OnceLock};

#[derive(Parser, Debug)]
#[command(name = "dots")]
struct Cli {
    #[arg(short, long)]
    /// Only output the found items
    silent: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Add the given path to the system
    #[command(arg_required_else_help = true)]
    Add {
        /// Format: (sub-dir of ~/.config/rebos/files)/(path to symlink).
        /// If the path is absolute, it is automatically prepended with <DEFAULT_SUBDIR>.
        /// "{hostname}" can be used as a placeholder for the actual hostname of the system.
        /// "{home}" can be used as a placeholder for the home dir of the user.
        path: PathBuf,

        #[arg(long)]
        /// Copy instead of symlink the path
        copy: bool,

        #[arg(short, long)]
        /// Overwrite the destination without asking
        force: bool,
    },
    /// Remove the given path from the system (does not remove the files the path points to, only the symlink)
    #[command(arg_required_else_help = true)]
    Remove {
        /// Format: (sub-dir of ~/.config/rebos/files}/{path to symlink)
        /// If the path is absolute, it is assumed to already be the path to remove.
        /// "{hostname}" can be used as a placeholder for the actual hostname of the system.
        /// "{home}" can be used as a placeholder for the home dir of the user.
        path: PathBuf,
    },
    /// Import the given path from the system
    #[command(arg_required_else_help = true)]
    Import {
        /// Format: (sub-dir of ~/.config/rebos/files)/(path to symlink).
        /// If the path is absolute, it is automatically prepended with <DEFAULT_SUBDIR>.
        /// "{hostname}" can be used as a placeholder for the actual hostname of the system.
        /// "{home}" can be used as a placeholder for the home dir of the user.
        path: PathBuf,

        #[arg(long)]
        /// Copy instead of symlink the path
        copy: bool,
    },
    /// Outputs a list of all symlinks on the system that are probably made by dots
    List {
        #[arg(short, long)]
        /// Assume that the current user is root
        rooted: bool,

        #[arg(long, trailing_var_arg = true, num_args(1..))]
        copy: Option<Vec<String>>,
    },
    /// Debugging commands
    #[command(subcommand)]
    Debug(DebugCommands),
}

#[derive(Subcommand, Debug)]
enum DebugCommands {
    /// Print the config path of the given path
    ConfigPath {
        /// Format: (sub-dir of ~/.config/rebos/files)/(path to symlink).
        /// If the path is absolute, it is automatically prepended with <DEFAULT_SUBDIR>.
        /// "{hostname}" can be used as a placeholder for the actual hostname of the system.
        /// "{home}" can be used as a placeholder for the home dir of the user.
        path: PathBuf,
    },
}

static SILENT: OnceLock<bool> = OnceLock::new();

fn main() {
    let args = Cli::parse();

    SILENT.set(args.silent).expect("Failed to set SILENT");

    match args.command {
        Commands::Add { path, force, copy } => add::add(&path, force, copy),
        Commands::Remove { path } => remove::remove(&path),
        Commands::Import { path, copy } => import::import(&path, copy),
        Commands::List { rooted, copy } => list::list(rooted, copy),
        Commands::Debug(debug_command) => debug::debug(debug_command),
    }
}
