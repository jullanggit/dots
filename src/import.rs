use std::{
    fs,
    io::{self},
    path::Path,
};

use crate::{
    add::add,
    util::{config_path, rerun_with_root_if_permission_denied, system_path},
};

/// Imports the given config path from the system path
pub fn import(cli_path: &Path, copy: bool) {
    let config_path = config_path(cli_path);
    let system_path = system_path(cli_path);

    if copy && system_path.is_dir() {
        panic!("Only files and symlinks are currently supported with --copy")
    }

    // Copy system path to config path
    let copy_result = if system_path.is_dir() {
        copy_dir(&system_path, &config_path)
    } else {
        fs::copy(&system_path, &config_path).map(|_| ())
    };

    rerun_with_root_if_permission_denied(
        copy_result,
        &format!(
            "copying system path ({}) to config path ({})",
            system_path.display(),
            config_path.display()
        ),
    );

    add(cli_path, true, copy);
}

/// Recursively copies the source directory to the target path
fn copy_dir(source: impl AsRef<Path>, target: impl AsRef<Path>) -> io::Result<()> {
    // Create destination
    fs::create_dir_all(&target)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;

        let entry_source_path = entry.path();
        let entry_target_path = target.as_ref().join(entry.file_name());

        if entry_source_path.is_dir() {
            copy_dir(entry_source_path, entry_target_path)?;
        } else {
            fs::copy(entry_source_path, entry_target_path)?;
        }
    }

    Ok(())
}
