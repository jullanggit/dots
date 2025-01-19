use std::{fs, io::ErrorKind, path::Path};

use crate::{
    add::add,
    util::{config_path, rerun_with_root, system_path},
};

/// Imports the given config path from the system path
pub fn import(cli_path: &Path) {
    let config_path = config_path(cli_path);
    let system_path = system_path(cli_path);

    // Copy system path to config path
    if let Err(e) = fs::copy(system_path, config_path) {
        match e.kind() {
            ErrorKind::PermissionDenied => rerun_with_root("Copying system path to config path"),
            other => panic!("Error copying system path to config path: {other}"),
        }
    }
    add(cli_path, true);
}
