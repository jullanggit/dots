use std::{fs::remove_file, path::Path};

use anyhow::{Context as _, Result};

use crate::util::system_path;

pub fn remove(path: &Path) -> Result<()> {
    let path = system_path(path)?;

    remove_file(path).context("deleting symlink")?;

    Ok(())
}
