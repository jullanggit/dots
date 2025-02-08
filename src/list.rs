use tokio::sync::Notify;

use crate::{
    config::CONFIG,
    util::{get_hostname, rerun_with_root_args, system_path},
};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

/// Prints all symlinks on the system, that are probably made by dots
pub fn list(rooted: bool) {
    // Start the tokio runtime
    tokio::runtime::Builder::new_multi_thread()
        .build()
        .unwrap()
        .block_on(async {
            // Rerun with root if required
            if CONFIG.root && !rooted {
                rerun_with_root_args(&["--rooted"]);
            }

            let items = Arc::new(Mutex::new(Vec::new()));

            // The amount of currently pending operations
            let pending = Arc::new(AtomicUsize::new(0));
            // The notification when no operations are pending anymore
            let notify = Arc::new(Notify::new());

            // Add initial paths
            for path in &CONFIG.list_paths {
                pending.fetch_add(1, Ordering::AcqRel);

                tokio::spawn(process_dir(
                    path.into(),
                    items.clone(),
                    pending.clone(),
                    notify.clone(),
                ));
            }

            // Wait for all operations to complete
            notify.notified().await;

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
        });
}

async fn process_dir(
    path: PathBuf,
    items: Arc<Mutex<Vec<PathBuf>>>,
    pending: Arc<AtomicUsize>,
    notify: Arc<Notify>,
) {
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
                // ...add the subpath to the items
                items.lock().unwrap().push(stripped.to_owned());
            }
        } else if file_type.is_dir() {
            // Recurse into the dir
            pending.fetch_add(1, Ordering::Release);

            tokio::spawn(process_dir(
                dir_entry.path(),
                items.clone(),
                pending.clone(),
                notify.clone(),
            ));
        }
    }

    // Remove ourselves from the pending, notify if we're the last one
    if pending.fetch_sub(1, Ordering::AcqRel) == 1 {
        notify.notify_waiters();
    }
}
