use tokio::{
    sync::{Mutex, Semaphore},
    task::LocalSet,
};

use crate::{
    config::CONFIG,
    util::{get_hostname, rerun_with_root_args, system_path},
};
use std::{collections::HashSet, path::PathBuf, sync::Arc};

static SEMAPHORE: Semaphore = Semaphore::const_new(100);

/// Prints all symlinks on the system, that are probably made by dots
pub async fn list(rooted: bool) {
    if CONFIG.root && !rooted {
        rerun_with_root_args(&["--rooted"]);
    }

    let items = Arc::new(Mutex::new(HashSet::new()));

    let local = LocalSet::new();
    for path in &CONFIG.list_paths {
        local.spawn_local(process_dir(path.into(), items.clone()));
    }
    local.await;

    for item in items.lock().await.iter() {
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

async fn process_dir(path: PathBuf, items: Arc<Mutex<HashSet<PathBuf>>>) {
    let local = LocalSet::new();
    {
        let _permit = SEMAPHORE.acquire().await.unwrap();
        // For each DirEntry under the path (ignoring errors using .flatten())
        let mut read_dir = tokio::fs::read_dir(path).await.unwrap();
        while let Some(dir_entry) = read_dir.next_entry().await.unwrap() {
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
                    items.lock().await.insert(stripped.to_owned());
                }
            } else if file_type.is_dir() {
                local.spawn_local(process_dir(dir_entry.path(), items.clone()));
            }
        }
    }
    local.await;
}
