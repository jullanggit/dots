use std::{fs::remove_file, io::ErrorKind, path::Path};

use crate::{rerun_with_root, system_path};

#[expect(clippy::wildcard_enum_match_arm)]
pub fn remove(path: &Path) {
    let path = system_path(path);
    if let Err(e) = remove_file(path) {
        match e.kind() {
            // Inform the user and retry with root privileges
            ErrorKind::PermissionDenied => {
                rerun_with_root("Deleting symlink");
            }
            other => panic!("Error deleting symlink: {other:?}"),
        }
    }
}
