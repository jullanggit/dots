use std::{
    collections::HashSet,
    fs::{self, FileType},
    io::ErrorKind,
    path::PathBuf,
    sync::Mutex,
};

use rayon::iter::{ParallelBridge, ParallelIterator};
use walkdir::WalkDir;

use crate::{
    config::CONFIG,
    util::{get_hostname, rerun_with_root, system_path},
};

/// Prints all symlinks on the system, that are probably made by dots
pub fn list() {
    let items = Mutex::new(HashSet::new());

    let mut paths_to_search: Vec<PathBuf> = CONFIG
        .list_paths
        .iter()
        .map(|string| string.into())
        .collect();

    let mut i = 0;
    while i < paths_to_search.len() {
        fs::read_dir(&paths_to_search[i])
            .unwrap()
            .flatten()
            .for_each(|dir_entry| {
                let file_type = dir_entry.file_type().unwrap();
                if file_type.is_symlink() {
                    // get its target
                    let target = fs::read_link(dir_entry.path()).expect("Failed to get target");
                    // If the target is in the files/ dir...
                    if let Ok(stripped) = target.strip_prefix(&CONFIG.files_path)
                        // ...and was plausibly created by dots...
                        && system_path(stripped) == dir_entry.path()
                    {
                        // ...add the subpath to the items
                        let mut items = items.lock().expect("Failed to lock items");
                        items.insert(stripped.to_owned());
                    }
                } else if file_type.is_dir() {
                    paths_to_search.push(dir_entry.path());
                }
            });
    }

    let items = items.lock().expect("Failed to lock items");
    for item in items.iter() {
        // Convert to a string, so strip_prefix() doesnt remove leading slashes
        let str = item.to_str().expect("Item should be valid UTF-8");

        let formatted = str
            .strip_prefix(&CONFIG.default_subdir) // If the subdir is the default one, remove it
            .map(Into::into)
            // If the subdir is the current hostname, replace it with {hostname}
            .or(str
                .strip_prefix(&get_hostname())
                .map(|str| format!("{{hostname}}{str}")))
            .unwrap_or(str.into());

        println!("{formatted}");
    }
}
