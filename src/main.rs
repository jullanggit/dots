#![feature(let_chains)]

mod add;
mod import;
mod list;
mod remove;
mod util;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "dots")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Add {
        /// Format: (sub-dir of ~/.config/rebos/files)/(path to symlink).
        /// If the path is absolute, it is automatically prepended with <DEFAULT_SUBDIR>
        /// "{hostname}" can be used as a placeholder for the actual hostname of the system
        path: PathBuf,

        #[arg(default_value_t = {"common".into()}, short, long)]
        default_subdir: String,

        #[arg(short, long)]
        /// Overwrite the destination without asking
        force: bool,
    },
    #[command(arg_required_else_help = true)]
    Remove {
        /// Format: (sub-dir of ~/.config/rebos/files}/{path to symlink)
        /// If the path is absolute, it is assumed to already be the path to remove
        /// "{hostname}" can be used as a placeholder for the actual hostname of the system
        path: PathBuf,
    },
    /// Import the given path from the system
    #[command(arg_required_else_help = true)]
    Import {
        /// Format: (sub-dir of ~/.config/rebos/files}/{path to symlink)
        /// If the path is absolute, it is assumed to already be the path to remove
        /// "{hostname}" can be used as a placeholder for the actual hostname of the system
        path: PathBuf,

        #[arg(default_value_t = {"common".into()}, short, long)]
        default_subdir: String,
    },
    /// Outputs a list of all symlinks on the system that are probably made by dots
    List,
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Add {
            path,
            default_subdir,
            force,
        } => add::add(&path, &default_subdir, force),
        Commands::Remove { path } => remove::remove(&path),
        Commands::Import {
            path,
            default_subdir,
        } => import::import(&path, &default_subdir),
        Commands::List => list::list(),
    }
}
