use anyhow::{Context as _, Result};

use crate::{
    config::CONFIG,
    util::{
        config_path, get_hostname, home, paths_equal, rerun_with_root_args,
        rerun_with_root_if_permission_denied, system_path,
    },
};
use std::{
    fs::{self},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

/// Prints all symlinks on the system, that are probably made by dots
#[expect(clippy::unwrap_used)] // Cant really handle errors in worker threads, we'd unwrap them at some point anyways
pub fn list(rooted: bool, copy: Option<Vec<String>>) -> Result<()> {
    if let Some(items) = copy {
        return list_copy(items);
    }

    // Rerun with root if required
    if CONFIG.root && !rooted {
        rerun_with_root_args(&["--rooted"]);
    }

    let threads = thread::available_parallelism().map_or(12, Into::into);

    // Set up pending paths
    let pending_paths = Mutex::new(
        CONFIG
            .list_paths
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
    );

    let pending = AtomicUsize::new(0);

    thread::scope(|scope| {
        for _ in 0..threads {
            #[expect(clippy::integer_division)]
            scope.spawn(|| {
                let mut local_pending_paths: Vec<PathBuf> = Vec::new();

                // Keep this here to avoid reallocating the vec for every path
                let mut paths_to_push = Vec::new();

                loop {
                    // Try the local queue
                    if let Some(path) = local_pending_paths.pop() {
                        // Add ourselves to pending
                        pending.fetch_add(1, Ordering::AcqRel);

                        // Process path
                        process_path(&mut paths_to_push, &path)
                            .with_context(|| format!("Failed to process path {}", path.display()))
                            .unwrap();

                        if !paths_to_push.is_empty() {
                            pending_paths.lock().unwrap().append(&mut paths_to_push);
                        }

                        // Remove ourselves from pending
                        pending.fetch_sub(1, Ordering::AcqRel);

                        continue;
                    }

                    // Try getting a batch of paths from the shared queue
                    {
                        let mut pending_paths = pending_paths.lock().unwrap();
                        if !pending_paths.is_empty() {
                            let len = pending_paths.len();
                            let num_take = (len / 2).max(1); // Take at least 1 element

                            // Add `num_take` paths to `local_pending_paths`
                            pending_paths
                                .drain(len - num_take..)
                                .collect_into(&mut local_pending_paths);
                        }
                    } // Drop the lock

                    // If no work is left, break
                    if local_pending_paths.is_empty() && pending.load(Ordering::Acquire) == 0 {
                        break;
                    }

                    // Avoid busy-looping
                    thread::yield_now();
                }
            });
        }
    });

    Ok(())
}

fn process_path(paths_to_push: &mut Vec<PathBuf>, path: &Path) -> Result<()> {
    if let Ok(read_dir) = fs::read_dir(path) {
        // Ignore errors with .flatten()
        for dir_entry in read_dir.flatten() {
            let entry_path = dir_entry.path();

            // Get the file type
            let file_type = dir_entry.file_type().with_context(|| {
                format!("Failed to get file type of '{}'", entry_path.display())
            })?;

            if file_type.is_symlink() {
                // get the entries target
                // Dont panic on failure
                if let Ok(target) = fs::read_link(&entry_path) {
                    // If the target is in the files/ dir...
                    if let Ok(stripped) = target.strip_prefix(&CONFIG.files_path)
                // ...and was plausibly created by dots...
                && system_path(stripped)? == dir_entry.path()
                    {
                        // Convert to a string, so strip_prefix() doesnt remove leading slashes
                        if let Some(str) = stripped.to_str() {
                            let str = str.replace(&home()?, "/{home}");

                            let formatted = str
                                .strip_prefix(&CONFIG.default_subdir) // If the subdir is the default one, remove it
                                .map(Into::into)
                                // If the subdir is the current hostname, replace it with {hostname}
                                .or_else(|| {
                                    str.strip_prefix(&get_hostname().ok()?)
                                        .map(|str| format!("{{hostname}}{str}"))
                                })
                                .unwrap_or(str);

                            println!("{formatted}");
                        }
                    }
                }
            } else if file_type.is_dir() {
                let path = dir_entry.path();

                // Filter out ignored paths
                if !CONFIG.ignore_paths.contains(&path) {
                    // Recurse into the dir
                    paths_to_push.push(path);
                }
            }
        }
    }

    Ok(())
}

fn list_copy(items: Vec<String>) -> Result<()> {
    for item in items {
        let path = Path::new(&item);

        let config_path = config_path(path)?;
        let system_path = system_path(path)?;

        // If path exists on the system
        if rerun_with_root_if_permission_denied(
            fs::exists(path),
            &format!("checking if the path {} already exists", path.display()),
            // And is equal to the one in the config
        )? && paths_equal(&config_path, &system_path).is_ok()
        {
            // Print it
            println!("{item}");
        }
    }

    Ok(())
}
