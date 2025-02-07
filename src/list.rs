use ignore::{WalkBuilder, WalkState};

use crate::{
    config::CONFIG,
    util::{get_hostname, rerun_with_root_args, system_path},
};
use std::{
    collections::{HashSet, VecDeque},
    fs::{self},
    sync::Mutex,
};

/// Prints all symlinks on the system, that are probably made by dots
pub fn list(rooted: bool) {
    if CONFIG.root && !rooted {
        rerun_with_root_args(&["--rooted"]);
    }

    let items = Mutex::new(HashSet::new());

    let mut walker = WalkBuilder::new(&CONFIG.list_paths[0]);
    for dir in CONFIG.list_paths.iter().skip(1) {
        walker.add(dir);
    }
    walker.follow_links(false);

    walker.ignore(false);
    walker.hidden(false);
    walker.git_ignore(false);
    walker.git_exclude(false);

    walker.build_parallel().run(|| {
        Box::new(|entry| {
            if let Ok(entry) = entry {
                // If the entry is a symlink...
                if entry.path_is_symlink() {
                    // ...get its target
                    let target = fs::read_link(entry.path()).expect("Failed to get target");
                    // If the target is in the files/ dir...
                    if let Ok(stripped) = target.strip_prefix(&CONFIG.files_path)
                        // ...and was plausibly created by dots...
                        && system_path(stripped) == entry.path()
                    {
                        // ...add the subpath to the items
                        let mut items = items.lock().expect("Failed to lock items");
                        items.insert(stripped.to_owned());
                    }
                }
            }
            WalkState::Continue
        })
    });

    for item in items.lock().unwrap().iter() {
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
