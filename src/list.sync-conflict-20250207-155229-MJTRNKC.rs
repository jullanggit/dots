use crate::{
    config::CONFIG,
    util::{get_hostname, rerun_with_root_args, system_path},
};
use std::{
    collections::{HashSet, VecDeque},
    fs::{self},
};

/// Prints all symlinks on the system, that are probably made by dots
pub fn list(rooted: bool) {
    if CONFIG.root && !rooted {
        rerun_with_root_args(&["--rooted"]);
    }

    let mut items = HashSet::new();

    // Create the queue
    let mut queue = VecDeque::from_iter(CONFIG.list_paths.iter().map(|string| string.into()));

    // While there are items to process
    while let Some(path) = queue.pop_front() {
        // For each DirEntry under the path (ignoring errors using .flatten())
        fs::read_dir(path).unwrap().flatten().for_each(|dir_entry| {
            let file_type = dir_entry.file_type().unwrap();
            if file_type.is_symlink() {
                // get the entries target
                let target = fs::read_link(dir_entry.path()).expect("Failed to get target");
                // If the target is in the files/ dir...
                if let Ok(stripped) = target.strip_prefix(&CONFIG.files_path)
                    // ...and was plausibly created by dots...
                    && system_path(stripped) == dir_entry.path()
                {
                    // ...add the subpath to the items
                    items.insert(stripped.to_owned());
                }
            } else if file_type.is_dir() {
                // Add the path to the queue
                queue.push_back(dir_entry.path());
            }
        });
    }

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
