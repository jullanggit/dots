use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::{
    config::CONFIG,
    util::{get_hostname, rerun_with_root_args, system_path},
};
use std::{path::PathBuf, sync::Arc};

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

            let num_sems = 900;

            let sem = Arc::new(Semaphore::new(num_sems));

            // Add initial paths
            for path in &CONFIG.list_paths {
                let permit = sem.clone().acquire_owned().await.unwrap();
                tokio::spawn(process_dir(path.into(), sem.clone(), permit));
            }

            // Try acquiring all permits, will only be successful once all threads have dropped theirs
            let _last = sem.acquire_many(num_sems as _).await.unwrap();
        });
}

// Pass the permit to the function, to avoid the delay between the function getting called
// and a permit being acquired, which can lead to inconsistencies
async fn process_dir(path: PathBuf, sem: Arc<Semaphore>, _permit: OwnedSemaphorePermit) {
    // Iterate over dir entries
    let mut read_dir = tokio::fs::read_dir(&path).await.unwrap();
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
            println!("i ran too");
            let permit = sem.clone().acquire_owned().await.unwrap();
            println!("i ran tooo");
            // Recurse into the dir
            tokio::spawn(process_dir(dir_entry.path(), sem.clone(), permit));
        }
    }
}
