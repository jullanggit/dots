use crate::{
    config::CONFIG,
    util::{
        config_path, get_hostname, paths_equal, rerun_with_root_args,
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
    vec: Mutex<Vec<PathBuf>>,
    /// the len of vec
    len: AtomicUsize,
    /// the amount of threads currently waiting to lock vec
    waiting: AtomicUsize,
}
impl PendingPaths {
    /// Pop from the vec
    /// Note that this may block the current thread
    fn push(&self, value: PathBuf) {
        self.vec.lock().unwrap().push(value);
        self.len.fetch_add(1, Ordering::AcqRel);
    }
    /// Pop from the vec
    /// Note that this may block the current thread
    fn pop(&self) -> Option<PathBuf> {
        if let Some(popped) = self.vec.lock().unwrap().pop() {
            // successful pop -> decrement self.len
            self.len.fetch_sub(1, Ordering::AcqRel);
            Some(popped)
        } else {
            None
        }
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
pub fn list(rooted: bool, copy: Option<Vec<String>>) {
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
    let pending_paths = Vec::from_iter(iter::repeat_with(PendingPaths::default).take(threads));
    for (index, path) in CONFIG.list_paths.iter().enumerate() {
        pending_paths[index].push(path.into());
    }

    let pending = AtomicUsize::new(0);

    // The borrow checker wont let us just capture i in 'for _ in ...', so we have to do this
    let index = AtomicUsize::new(0);

    thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                let thread_index = index.fetch_add(1, Ordering::Relaxed);

                'process_paths: loop {
                    // Try getting an element from the current thread's vec
                    let current_option_path = pending_paths[thread_index].pop();

                    match current_option_path {
                        Some(path) => {
                            process_path(&pending_paths, &pending, thread_index, &path);
                        }
                        None => {
                            // Try getting one from another thread
                            'find_path_to_process: loop {
                                // (thread, wait)
                                let mut with_wait: Option<Vec<(usize, usize)>> = None;

                                let mut no_wait: Option<usize> = None;

                                // For all other threads
                                for (other_index, other_pending_paths) in
                                    pending_paths.iter().enumerate()
                                {
                                    // Skip checking ourselves
                                    if other_index == thread_index {
                                        continue;
                                    }

                                    // If the other thread's vec has items
                                    if other_pending_paths.len() != 0 {
                                        let waiting = other_pending_paths.waiting();

                                        // If no one is currently waiting
                                        if waiting == 0 {
                                            no_wait = Some(other_index);
                                            break;
                                        // If someone is waiting
                                        } else {
                                            // Add to with_wait
                                            with_wait
                                                .get_or_insert_default()
                                                .push((other_index, waiting));
                                        }
                                    }
                                }

                                let to_process = match (no_wait, with_wait) {
                                    // No wait
                                    (Some(other_thread), _) => Some(other_thread),

                                    // With wait, get the thread with the smallest waiting
                                    (None, Some(possible_other_threads)) => Some(
                                        possible_other_threads
                                            .iter()
                                            .min_by_key(|(_thread, waiting)| waiting)
                                            .map(|(thread, _waiting)| *thread)
                                            .expect("with_wait shouldn't be empty"),
                                    ),

                                    // No threads with items
                                    (None, None) => None,
                                };

                                if let Some(other_index) = to_process {
                                    let other_pending_paths = &pending_paths[other_index];

                                    // start waiting -> pop -> stop waiting
                                    other_pending_paths.start_waiting();

                                    let other_option_path = other_pending_paths.pop();

                                    other_pending_paths.stop_waiting();

                                    if let Some(path) = other_option_path {
                                        process_path(&pending_paths, &pending, thread_index, &path);
                                        break 'find_path_to_process;
                                    }
                                }

                                // If there are no pending paths and no paths are left, stop processing
                                let pending = pending.load(Ordering::Acquire);
                                if pending == 0 && to_process.is_none() {
                                    break 'process_paths;
                                }
                            }
                        }
                    }
                }
            });
        }
    });
}

fn process_path(
    pending_paths: &[PendingPaths],
    pending: &AtomicUsize,
    thread_index: usize,
    path: &Path,
) {
    // Add ourselves to pending
    pending.fetch_add(1, Ordering::AcqRel);

    // Ignore errors with .flatten()
    for dir_entry in fs::read_dir(path).unwrap().flatten() {
        // Get the file type
        let file_type = dir_entry.file_type().unwrap();

        if file_type.is_symlink() {
            // get the entries target
            let target = fs::read_link(dir_entry.path()).expect("Failed to get target");
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
            let path = dir_entry.path();

            // Recurse into the dir
            pending_paths[thread_index].push(path);
        }
    }

    // Remove ourselves from pending
    pending.fetch_sub(1, Ordering::AcqRel);
}

fn list_copy(items: Vec<String>) {
    for item in items {
        let path = Path::new(&item);

        let config_path = config_path(path);
        let system_path = system_path(path);

        // If path exists on the system
        if rerun_with_root_if_permission_denied(
            fs::exists(path),
            &format!("checking if the path {} already exists", path.display()),
            // And is equal to the one in the config
        ) && paths_equal(&config_path, system_path).is_ok()
        {
            // Print it
            println!("{item}");
        };
    }
}
