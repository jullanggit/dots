use std::{fs::remove_file, path::Path};

use anyhow::Result;

use crate::util::{rerun_with_root_if_permission_denied, system_path};

pub fn remove(path: &Path) -> Result<()> {
    let path = system_path(path)?;

    rerun_with_root_if_permission_denied(remove_file(path), "deleting symlink")?;

    Ok(())
}
