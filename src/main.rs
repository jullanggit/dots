use std::{
    env,
    fs::{create_dir_all, remove_file, symlink_metadata},
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
        eprintln!("Default subdir is not allowed to be absolute");
        exit(1);
    }

    let home = env::var("HOME").expect("HOME env variable not set");

    let mut origin = PathBuf::from(home);
    origin.push(".config/rebos/files/");

    if path.starts_with("/") {
        origin.push(default_subdir);

        path = path
            .strip_prefix("/")
            .expect("Checked that path starts with a '/'");
    }
    origin.push(path);
    origin
}

fn add(path: &Path, default_subdir: &str) {
    let origin = get_origin(path, default_subdir);
    let link = trim_files_subdir(path);

    // Check if the path already exists
    while let Ok(metadata) = symlink_metadata(&link) {
        // Check if it is a symlink
        if metadata.is_symlink() {
            return;
        }
        // Ask for retry, if not, abort
        if bool_question(&format!(
            "The path {} already exists and isn't a symlink, retry?",
            link.display()
        )) {
            continue;
        }
        exit(1)
    }
    println!("Symlinking {}", link.display());
    create_symlink(&origin, &link);
}

#[expect(clippy::wildcard_enum_match_arm)]
fn create_symlink(origin: &Path, link: &Path) {
    if let Err(e) = symlink(origin, link) {
        match e.kind() {
            ErrorKind::PermissionDenied => {
                println!("Insufficient permissions to create the symlink");
                exit(1) // Exit, so that it can be retried with adequate permissions
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
                            ErrorKind::PermissionDenied => {
                                println!("Insufficient permissions to create parent directories");
                                exit(1)
                            }
                            other => println!("Error creating parent directory: {other:?}"),
                        }
                    } else {
                        create_symlink(origin, link);
                    }
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
            ErrorKind::PermissionDenied => {
                println!("Insufficient permissions to delete symlink");
                exit(1)
            }
            other => println!("Error deleting symlink: {other:?}"),
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

#[expect(clippy::let_underscore_must_use)]
fn bool_question(question: &str) -> bool {
    print!("{question} ");
    let _ = stdout().flush();

    let mut buffer = String::new();

    loop {
        buffer.clear();
        let _ = stdin().read_line(&mut buffer);

        match buffer.trim() {
            "y" | "Y" | "yes" | "Yes" => return true,
            "n" | "N" | "no" | "No" => return false,
            _other => continue,
        }
    }
}
