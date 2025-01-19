use std::{collections::HashSet, fs, sync::Mutex};

use rayon::iter::{ParallelBridge, ParallelIterator};
use walkdir::WalkDir;

use crate::{
    config::CONFIG,
    util::{files_path, get_hostname, system_path},
};

/// Prints all symlinks on the system, that are probably made by dots
pub fn list() {
    let files_path = files_path();

    let items = Mutex::new(HashSet::new());

    CONFIG
        .list_paths
        .iter()
        .flat_map(|root_path| WalkDir::new(root_path).into_iter().flatten())
        .par_bridge()
        .for_each(|entry| {
            // If the entry is a symlink...
            if entry.path_is_symlink() {
                // ...get its target
                let target = fs::read_link(entry.path()).expect("Failed to get target");
                // If the target is in the files/ dir...
                if let Ok(stripped) = target.strip_prefix(&files_path)
                    // ...and was plausibly created by dots...
                    && system_path(stripped) == entry.path()
                {
                    // ...add the subpath to the items
                    let mut items = items.lock().expect("Failed to lock items");
                    items.insert(stripped.to_owned());
                }
            }
        });

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
