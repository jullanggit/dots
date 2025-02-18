use std::{
    fs::{self, create_dir_all, symlink_metadata},
    io::{ErrorKind, Write as _, stdin, stdout},
    os::unix::fs::symlink,
    path::Path,
    process::exit,
};

use color_eyre::{
    Result,
    eyre::{OptionExt as _, eyre},
};

use crate::util::{
    config_path, paths_equal, rerun_with_root, rerun_with_root_if_permission_denied, system_path,
};

/// Symlink a the given path to its location in the actual system
pub fn add(path: &Path, force: bool, copy: bool) -> Result<()> {
    if copy {
        return add_copy(path, force);
    }

    let config_path = config_path(path)?;
    let system_path = system_path(path)?;

    // If the path already exists
    if symlink_metadata(&system_path).is_ok() {
        // Check if it is a symlink that points to the correct location
        if let Ok(destination) = fs::read_link(&system_path)
            && destination == config_path
        {
            return Ok(());
        }

        // -> It isnt
        ask_for_overwrite(force, &system_path)?;
    }

    // At this point the path either doesn't exist yet, or the user has decided to overwrite it
    println!(
        "Symlinking {} to {}",
        config_path.display(),
        system_path.display(),
    );
    create_symlink(&config_path, &system_path)
}

/// Symlink a the given path to its location in the actual system
pub fn add_copy(path: &Path, force: bool) -> Result<()> {
    let config_path = config_path(path)?;
    let system_path = system_path(path)?;

    if config_path.is_dir() {
        return Err(eyre!(
            "Only files and symlinks are currently supported with --copy"
        ));
    }

    // If path exists on the system
    if rerun_with_root_if_permission_denied(
        fs::exists(path),
        &format!("checking if the path {} already exists", path.display()),
        // And is not equal to the one in the config
    )? && let Err(e) = paths_equal(&config_path, &system_path)
    {
        eprintln!("{e}");
        ask_for_overwrite(force, &system_path)?;
    }

    // At this point the path either doesn't exist yet, or the user has decided to overwrite it
    println!(
        "Copying {} to {}",
        config_path.display(),
        system_path.display(),
    );
    rerun_with_root_if_permission_denied(
        fs::copy(config_path, system_path).map(|_| {}), // Ignore number of bytes copied
        "copying config path to system path",
    )
}

/// Asks for overwrite and removes the path from the system if requested, exits if not
fn ask_for_overwrite(force: bool, system_path: &Path) -> Result<()> {
    if force
        || bool_question(&format!(
            "The path {} already exists, overwrite?",
            system_path.display()
        )) && bool_question("Are you sure?")
    {
        let result = if system_path.is_dir() {
            fs::remove_dir_all(system_path)
        } else {
            fs::remove_file(system_path)
        };

        rerun_with_root_if_permission_denied(result, "removing path")
    } else {
        exit(1)
    }
}

/// Creates a symlink from `config_path` to `system_path`
#[expect(clippy::wildcard_enum_match_arm)]
fn create_symlink(config_path: &Path, system_path: &Path) -> Result<()> {
    // Try creating the symlink
    if let Err(e) = symlink(config_path, system_path) {
        match e.kind() {
            ErrorKind::PermissionDenied => {
                rerun_with_root("Creating symlink");
            }
            ErrorKind::NotFound => {
                rerun_with_root_if_permission_denied(
                    create_dir_all(
                        system_path
                            .parent()
                            .ok_or_eyre("Failed to get parent of system path")?,
                    ),
                    "creating parent directories",
                )?;

                create_symlink(config_path, system_path)?;
            }
            _ => {
                return Err(e.into());
            }
        }
    }
    Ok(())
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
            _other => {}
        }
    }
}
