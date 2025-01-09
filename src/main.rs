#![feature(let_chains)]

use std::{
    env::{self, current_exe},
    fs::{self, create_dir_all, remove_file, symlink_metadata},
    io::{stdin, stdout, ErrorKind, Write},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::{exit, Command},
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
fn config_path(mut cli_path: &Path, default_subdir: &str) -> PathBuf {
    if Path::new(default_subdir).is_absolute() {
        error_with_message("Default subdir is not allowed to be absolute");
    }

    // Get the users home directory
    let home = env::var("HOME").expect("HOME env variable not set");

    let mut config_path = PathBuf::from(format!("{home}.config/rebos/files"));

    // If the path started with "/", the default subdir was elided
    if let Ok(relative_path) = cli_path.strip_prefix("/") {
        // So we add it
        config_path.push(default_subdir);

        // And replace the absolute path with the relative one to avoid overwriting the entire config_path
        cli_path = relative_path
    }

    // Replace "{hostname}" with the actual hostname
    if let Ok(stripped_path) = cli_path.strip_prefix("{hostname}") {
        config_path.push(env::var("hostname").expect("Failed to get hostname"));

        cli_path = stripped_path;
    }

    config_path.push(cli_path);

    config_path
}

/// Symlink a the given path to its location in the actual system
fn add(path: &Path, default_subdir: &str) {
    let config_path = config_path(path, default_subdir);
    let system_path = system_path(path);

    // If the path already exists
    if symlink_metadata(system_path).is_ok() {
        // Check if it is a symlink that points to the correct location
        if let Ok(destination) = fs::read_link(system_path)
            && destination == config_path
        {
            return;
        }

        // -> It isnt
        // Ask if the file should be overwritten
        if !bool_question(&format!(
            "The path {} already exists, overwrite?",
            system_path.display()
        )) || !bool_question("Are you sure?")
        {
            exit(1)
        }
    }

    // At this point the path either doesn't exist yet, or the user has decided to overwrite it
    println!(
        "Symlinking {} to {}",
        system_path.display(),
        config_path.display()
    );
    create_symlink(&config_path, system_path);
}

/// Creates a symlink from `config_path` to `system_path`
fn create_symlink(config_path: &Path, system_path: &Path) {
    // Try creating the symlink
    if let Err(e) = symlink(config_path, system_path) {
        match e.kind() {
            ErrorKind::PermissionDenied => {
                rerun_with_root("Creating symlink");
            }
            ErrorKind::NotFound => {
                if let Err(e) =
                    create_dir_all(system_path.parent().expect("Path should have a parent"))
                {
                    match e.kind() {
                        ErrorKind::PermissionDenied => {
                            rerun_with_root("Creating parent directories");
                        }
                        other => error_with_message(&format!(
                            "Error creating parent directory: {other:?}"
                        )),
                    }
                } else {
                    create_symlink(config_path, system_path);
                }
            }
            other => {
                println!("Error creating symlink: {other:?}");
            }
        }
    };
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

#[expect(clippy::wildcard_enum_match_arm)]
fn remove(path: &Path) {
    let path = system_path(path);
    if let Err(e) = remove_file(path) {
        match e.kind() {
            // Inform the user and retry with root privileges
            ErrorKind::PermissionDenied => {
                rerun_with_root("Deleting symlink");
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
