use tokio::task::JoinSet;

use crate::{
    config::CONFIG,
    util::{get_hostname, rerun_with_root_args, system_path},
};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};

/// Prints all symlinks on the system, that are probably made by dots
pub fn list(rooted: bool) {
    if CONFIG.root && !rooted {
        rerun_with_root_args(&["--rooted"]);
    }

    let items = Arc::new(Mutex::new(HashSet::new()));

    // Create the workque, using a sender/receiver channel
    let (sender, receiver) = flume::unbounded();

    // Send initial root paths
    // TODO: Run with root if necessary (make this a config option)
    for root_path in &CONFIG.list_paths {
        sender.send(root_path.into()).unwrap();
    }

    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let mut join_set = JoinSet::new();

            for _ in 0..10 {
                let receiver = receiver.clone();
                let sender = sender.clone();
                let items = items.clone();
                join_set.spawn(async move {
                    // While the channel is open, wait for a path
                    while let Ok(path) = receiver.recv_timeout(Duration::from_millis(5)) {
                        // For each DirEntry under the path (ignoring errors using .flatten())
                        let mut read_dir = tokio::fs::read_dir(path).await.unwrap();

                        while let Ok(Some(dir_entry)) = read_dir.next_entry().await {
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
                                    let mut items = items.lock().expect("Failed to lock items");
                                    items.insert(stripped.to_owned());
                                }
                            } else if file_type.is_dir() {
                                // Add the path to the queue
                                sender.send(dir_entry.path()).unwrap();
                            }
                        }
                    }
                });
            }
            drop(sender);

            join_set.join_all().await;
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
