#![feature(let_chains)]

mod add;
mod list;
mod remove;
mod util;

use clap::{Parser, Subcommand};
use std::{
    env::{self, current_exe},
    path::{Path, PathBuf},
    process::{exit, Command},
};

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
    },
    #[command(arg_required_else_help = true)]
    Remove {
        /// Format: (sub-dir of ~/.config/rebos/files}/{path to symlink)
        /// If the path is absolute, it is assumed to already be the path to remove
        /// "{hostname}" can be used as a placeholder for the actual hostname of the system
        path: PathBuf,
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
        } => add::add(&path, &default_subdir),
        Commands::Remove { path } => remove::remove(&path),
        Commands::List => list::list(),
    }
}

/// Inform the user of the `failed_action` and rerun with root privileges
fn rerun_with_root(failed_action: &str) -> ! {
    println!("{failed_action} requires root privileges",);

    // Collect args
    let mut args: Vec<_> = env::args().collect();

    // Overwrite the exe path with the absolute path if possible
    if let Some(absolute_path) = current_exe()
        .ok()
        .and_then(|path| path.to_str().map(|path| path.to_owned()))
    {
        args[0] = absolute_path;
    }

    let home = env::var("HOME").expect("HOME env variable not set");

    let status = Command::new("/usr/bin/sudo")
        // Preserve $HOME
        .arg(format!("HOME={home}"))
        .args(args)
        .spawn()
        .expect("Failed to spawn child process")
        .wait()
        .expect("Failed to wait on child process");

    if !status.success() {
        exit(status.code().unwrap_or(1));
    } else {
        exit(0);
    }
}

/// Converts the path relative to files/ to the location on the actual system. (by trimming the subdir of files/ away)
fn system_path(path: &Path) -> &Path {
    if path.is_relative() {
        let str = path.as_os_str().to_str().unwrap();

        // Only keep the path from the first /
        Path::new(&str[str.find('/').unwrap()..])
    } else {
        // The default subdir was elided, so the path is already the correct one
        path
    }
}

// Not sure if theres a builtin for this
fn error_with_message(message: &str) -> ! {
    eprintln!("{message}");
    exit(1)
}
