use std::{
    fs,
    io::{self, ErrorKind},
    path::Path,
};

use crate::{
    add::add,
    util::{config_path, rerun_with_root, system_path},
};

/// Imports the given config path from the system path
pub fn import(cli_path: &Path) {
    let config_path = config_path(cli_path);
    let system_path = system_path(cli_path);

    // Copy system path to config path
    let copy_result = if system_path.is_dir() {
        copy_dir(system_path, &config_path)
    } else {
        fs::copy(system_path, &config_path).map(|_| ())
    };

    if let Err(e) = copy_result {
        match e.kind() {
            ErrorKind::PermissionDenied => rerun_with_root("Copying system path to config path"),
            other => panic!(
                "Error copying system path ({}) to config path ({}): {other}",
                system_path.display(),
                config_path.display()
            ),
        }
    }
    add(cli_path, true);
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
