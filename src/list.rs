use tokio::sync::{Notify, Semaphore};

use crate::{
    config::CONFIG,
    util::{config_path, get_hostname, paths_equal, rerun_with_root_args, system_path},
};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

/// Prints all symlinks on the system, that are probably made by dots
pub fn list(rooted: bool, copy: Option<Vec<String>>) {
    if let Some(items) = copy {
        return list_copy(items);
    }

    // Start the tokio runtime
    tokio::runtime::Builder::new_multi_thread()
        .build()
        .unwrap()
        .block_on(async {
            // Rerun with root if required
            if CONFIG.root && !rooted {
                rerun_with_root_args(&["--rooted"]);
            }

            // The amount of currently pending operations
            let pending = Arc::new(AtomicUsize::new(0));
            // The notification when no operations are pending anymore
            let notify = Arc::new(Notify::new());

            // Represents the max number of concurrently open fd's
            let sem = Arc::new(Semaphore::new(900));

            // Add initial paths
            for path in &CONFIG.list_paths {
                // Increment the number of pending operations
                pending.fetch_add(1, Ordering::AcqRel);

                tokio::spawn(process_dir(
                    path.into(),
                    pending.clone(),
                    notify.clone(),
                    sem.clone(),
                ));
            }

            // Wait for all operations to complete
            notify.notified().await;
        });
}

// Recursively processes the given path, printing found items to stdout
async fn process_dir(
    path: PathBuf,
    pending: Arc<AtomicUsize>,
    notify: Arc<Notify>,
    sem: Arc<Semaphore>,
) {
    // Avoid hitting the fd limit.
    // Is dropped at the end of the scope
    let _permit = sem.acquire().await.unwrap();

    // Iterate over dir entries
    let mut read_dir = tokio::fs::read_dir(path).await.unwrap();
    while let Some(dir_entry) = read_dir.next_entry().await.unwrap() {
        // Get the file type
        let file_type = dir_entry.file_type().await.unwrap();

        if file_type.is_symlink() {
            // get the entries target
            let target = tokio::fs::read_link(dir_entry.path())
                .await
                .expect("Failed to get target");
            // If the target is in the files/ dir...
            if let Ok(stripped) = target.strip_prefix(&CONFIG.files_path)
                // ...and was plausibly created by dots...
                && system_path(stripped) == dir_entry.path()
            {
                // Convert to a string, so strip_prefix() doesnt remove leading slashes
                let str = stripped.to_str().expect("Item should be valid UTF-8");

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
        } else if file_type.is_dir() {
            // Increment the number of pending operations
            pending.fetch_add(1, Ordering::Release);

            // Recurse into the dir
            tokio::spawn(process_dir(
                dir_entry.path(),
                pending.clone(),
                notify.clone(),
                sem.clone(),
            ));
        }
    }

    // Decrement the number of pending operations
    // Notify if we're the last operation
    if pending.fetch_sub(1, Ordering::AcqRel) == 1 {
        notify.notify_waiters();
    }
}

fn list_copy(items: Vec<String>) {
    for item in items {
        let path = Path::new(&item);

        let config_path = config_path(path);
        let system_path = system_path(path);

        if paths_equal(&config_path, system_path).is_err() {
            println!("{item}");
        }
    }
}
