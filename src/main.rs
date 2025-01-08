#![feature(let_chains)]

use std::{
    env,
    fs::{self, create_dir_all, remove_file, symlink_metadata},
    io::{stdin, stdout, ErrorKind, Write},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::exit,
};

use clap::{Parser, Subcommand};

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
        /// Format: {sub-dir of ~/.config/rebos/files}/{path to symlink}.
        /// If the path is absolute, it is automatically prepended with <DEFAULT_SUBDIR>
        path: PathBuf,

        #[arg(default_value_t = {"common".into()}, short, long)]
        default_subdir: String,
    },
    #[command(arg_required_else_help = true)]
    Remove {
        /// Format: {sub-dir of ~/.config/rebos/files}/{path to symlink}
        /// If the path is absolute, it is assumed to already be the path to remove, without trimming
        path: PathBuf,
    },
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Add {
            path,
            default_subdir,
        } => add(&path, &default_subdir),
        Commands::Remove { path } => remove(&path),
    }
}

/// Converts the path that should be symlinked to the path in the files/ directory
fn config_path(mut path: &Path, default_subdir: &str) -> PathBuf {
    if Path::new(default_subdir).is_absolute() {
        error_with_message("Default subdir is not allowed to be absolute");
    }

    // Get the users home directory
    let home = env::var("HOME").expect("HOME env variable not set");

    let mut config_path = PathBuf::from(home);
    config_path.push(".config/rebos/files/"); // And push the files/ directory onto it

    // If the path started with "/", the default subdir was elided
    if let Ok(relative_path) = path.strip_prefix("/") {
        // So we add it
        config_path.push(default_subdir);

        // And replace the absolute path with the relative one to avoid overwriting the entire config_path
        path = relative_path
    }
    config_path.push(path);

    config_path
}

/// Symlink a the given path to its location in the actual system
fn add(path: &Path, default_subdir: &str) {
    let config_path = config_path(path, default_subdir);
    let system_path = system_path(path);

    // Check if the path already exists
    while let Ok(metadata) = symlink_metadata(system_path) {
        // Check if it is a symlink and already points to the correct location
        if let Ok(destination) = fs::read_link(system_path)
            && destination == config_path
        {
            return;
        }

        // Ask for retry, if not, abort
        if bool_question(&format!(
            "The path {} already exists, retry?",
            system_path.display()
        )) {
            continue;
        } else {
            exit(1)
        }
    }
    println!("Symlinking {}", system_path.display());
    create_symlink(&config_path, system_path);
}

#[expect(clippy::wildcard_enum_match_arm)]
fn create_symlink(origin: &Path, link: &Path) {
    // Try creating the symlink
    if let Err(e) = symlink(origin, link) {
        match e.kind() {
            ErrorKind::PermissionDenied => {
                error_with_message("Insufficient permissions to create the symlink");
            }
            ErrorKind::NotFound => {
                if bool_question(&format!(
                    "Could not find the path {}, should the parent paths be created?",
                    link.display()
                )) {
                    if let Err(e) =
                        create_dir_all(link.parent().expect("Path shouldnt be just root or empty"))
                    {
                        match e.kind() {
                            // Inform the user and retry with root privileges
                            ErrorKind::PermissionDenied => {
                                println!("Creating parent directories requires root privileges",);
                                sudo::with_env(&["HOME"])
                                    .expect("Failed to acquire root privileges");
                            }
                            other => error_with_message(&format!(
                                "Error creating parent directory: {other:?}"
                            )),
                        }
                    } else {
                        create_symlink(origin, link);
                    }
                // Parent paths shouldnt be created
                } else {
                    exit(1)
                }
            }
            other => {
                println!("Error creating symlink: {other:?}");
            }
        }
    };
}

#[expect(clippy::wildcard_enum_match_arm)]
fn remove(path: &Path) {
    let path = system_path(path);
    if let Err(e) = remove_file(path) {
        match e.kind() {
            // Inform the user and retry with root privileges
            ErrorKind::PermissionDenied => {
                println!("Deleting symlink requires root privileges",);
                sudo::with_env(&["HOME"]).expect("Failed to acquire root privileges");
            }
            other => error_with_message(&format!("Error deleting symlink: {other:?}")),
        }
    }
}

/// Converts the path relative to files/ to the location on the actual system. (by trimming the subdir of files/ away)
fn system_path(path: &Path) -> &Path {
    if path.is_relative() {
        // Skip the first component (the subdir of files/)
        let mut components = path.components();
        components
            .next()
            .expect("Path should have at least one component");

        components.as_path()
    } else {
        // The default subdir was elided, so the path is already the correct one
        path
    }
}

/// Asks the user the given question and returns the users answer.
/// Returns false if getting the answer failed
fn bool_question(question: &str) -> bool {
    print!("{question} ");

    if stdout().flush().is_err() {
        return false;
    }

    let mut buffer = String::with_capacity(3); // The longest accepted answer is 3 characters long

    loop {
        buffer.clear();

        if stdin().read_line(&mut buffer).is_err() {
            return false;
        }

        match buffer.trim() {
            "y" | "Y" | "yes" | "Yes" => return true,
            "n" | "N" | "no" | "No" => return false,
            _other => continue,
        }
    }
}

// Not sure if theres a builtin for this
fn error_with_message(message: &str) -> ! {
    eprintln!("{message}");
    exit(1)
}
