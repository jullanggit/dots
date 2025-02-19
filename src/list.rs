use color_eyre::eyre::{Context as _, Result};

use crate::{
    config::CONFIG,
    util::{
        config_path, get_hostname, home, paths_equal, rerun_with_root_args,
        rerun_with_root_if_permission_denied, system_path,
    },
};
use std::{
    fs::{self},
    iter,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

#[derive(Default)]
struct PendingPaths {
    queue: Mutex<Vec<PathBuf>>,
    /// the len of the queue
    len: AtomicUsize,
    /// the amount of threads currently waiting to lock the queue
    waiting: AtomicUsize,
}
impl PendingPaths {
    /// Push to the queue.
    /// Note that this may block the current thread
    #[expect(clippy::expect_used)] // We only panic if another thread already did
    fn push(&self, value: PathBuf) {
        self.queue
            .lock()
            .expect("No other threads should panic")
            .push(value);
        self.len.fetch_add(1, Ordering::AcqRel);
    }
    /// Pop from the queue.
    /// Note that this may block the current thread
    #[expect(clippy::expect_used)] // We only panic if another thread already did
    fn pop(&self) -> Option<PathBuf> {
        self.queue
            .lock()
            .expect("No other threads should panic")
            .pop()
            .inspect(|_| {
                // successful pop -> decrement self.len
                self.len.fetch_sub(1, Ordering::AcqRel);
            })
    }
    fn len(&self) -> usize {
        self.len.load(Ordering::Acquire)
    }
    fn waiting(&self) -> usize {
        self.waiting.load(Ordering::Acquire)
    }
    fn start_waiting(&self) {
        self.waiting.fetch_add(1, Ordering::AcqRel);
    }
    fn stop_waiting(&self) {
        self.waiting.fetch_sub(1, Ordering::AcqRel);
    }
}

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
    // Each thread has its own vec, to which it will push new paths
    // If a thread's own vec is empty, it will try to get items from another thread's vec
    // We keep an external atomic len for each vec, so the threads dont have to lock the mutex to see if there are any elements
    // Additionally we keep a waiting field, so that threads can choose the least waited for vec
    let pending_paths: Vec<_> = iter::repeat_with(PendingPaths::default)
        .take(threads)
        .collect();
    for (index, path) in CONFIG.list_paths.iter().enumerate() {
        pending_paths[index].push(path.into());
    }

    let pending = AtomicUsize::new(0);

    // The borrow checker wont let us just capture i in 'for _ in ...', so we have to do this
    let index = AtomicUsize::new(0);

    thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                let my_index = index.fetch_add(1, Ordering::Relaxed); // We dont care about ordering

                loop {
                    // Try our own queue
                    if let Some(path) = pending_paths[my_index]
                        .pop()
                        // Or try stealing a path from another thread's queue
                        .or_else(|| try_steal_path(&pending_paths, my_index))
                    {
                        process_path(&pending_paths, &pending, my_index, &path)
                            .wrap_err_with(|| format!("Failed to process path {}", path.display()))
                            .unwrap();
                        continue;
                    }

                    // If no work is left, break
                    if pending.load(Ordering::Acquire) == 0 {
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

/// Try to steal a pending path from another thread.
fn try_steal_path(pending_paths: &[PendingPaths], my_index: usize) -> Option<PathBuf> {
    let mut candidate: Option<(usize, usize)> = None; // (thread_index, waiting)

    // For all other threads
    for (index, pending_paths) in pending_paths.iter().enumerate() {
        // Skip ourselves
        if index == my_index {
            continue;
        }

        // If the other thread's queue has items
        if pending_paths.len() > 0 {
            let waiting = pending_paths.waiting();

            let this_candidate = Some((index, waiting));

            // If no one is currently waiting
            if waiting == 0 {
                // Immediately choose this thread
                candidate = this_candidate;
                break;
            }

            // Otherwise, choose the candidate with the smallest waiting count
            candidate = match candidate {
                None => this_candidate,
                Some((_, current_waiting)) if waiting < current_waiting => this_candidate,
                other => other,
            };
        }
    }

    if let Some((other_index, _)) = candidate {
        let other_pending_paths = &pending_paths[other_index];

        // start waiting -> pop -> stop waiting
        other_pending_paths.start_waiting();
        let stolen = other_pending_paths.pop();
        other_pending_paths.stop_waiting();

        stolen
    } else {
        None
    }
}

fn process_path(
    pending_paths: &[PendingPaths],
    pending: &AtomicUsize,
    thread_index: usize,
    path: &Path,
) -> Result<()> {
    // Add ourselves to pending
    pending.fetch_add(1, Ordering::AcqRel);

    if let Ok(read_dir) = fs::read_dir(path) {
        // Ignore errors with .flatten()
        for dir_entry in read_dir.flatten() {
            let entry_path = dir_entry.path();

            // Get the file type
            let file_type = dir_entry.file_type().wrap_err_with(|| {
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
                    pending_paths[thread_index].push(path);
                }
            }
        }
    }

    // Remove ourselves from pending
    pending.fetch_sub(1, Ordering::AcqRel);

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
