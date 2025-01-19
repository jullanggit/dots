use std::{
    fs::{self, create_dir_all, symlink_metadata},
    io::{stdin, stdout, ErrorKind, Write},
    os::unix::fs::symlink,
    path::Path,
    process::exit,
};

use crate::util::{config_path, rerun_with_root, system_path};

/// Symlink a the given path to its location in the actual system
pub fn add(path: &Path, force: bool) {
    let config_path = config_path(path);
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
        if force
            || bool_question(&format!(
                "The path {} already exists, overwrite?",
                system_path.display()
            )) && bool_question("Are you sure?")
        {
            if system_path.is_dir() {
                fs::remove_dir_all(system_path).expect("Failed to remove path");
            } else {
                fs::remove_file(system_path).expect("Failed to remove path");
            }
        } else {
            exit(1)
        }
    }

    // At this point the path either doesn't exist yet, or the user has decided to overwrite it
    println!(
        "Symlinking {} to {}",
        config_path.display(),
        system_path.display(),
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
                        other => panic!("Error creating parent directory: {other}"),
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
