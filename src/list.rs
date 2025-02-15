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
    // Additionally we keep a waiting field, so that not all of the threads wait for the first available one
    let pending_paths = Vec::from_iter(
        iter::repeat_with(|| {
            (
                Mutex::new(Vec::new()),
                AtomicUsize::new(0),
                AtomicUsize::new(0),
            )
        })
        .take(threads),
    );
    for (index, path) in CONFIG.list_paths.iter().enumerate() {
        let (vec, len, _waiting) = &pending_paths[index];

        // Unwrap is fine, because were still single-threaded, so the lock can't be poisoned
        vec.lock().unwrap().push(path.into());

        len.fetch_add(1, Ordering::Relaxed);
    }

    let pending = AtomicUsize::new(0);

    // The borrow checker wont let us just capture i in 'for _ in ...', so we have to do this
    let index = AtomicUsize::new(0);

    thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                let thread_index = index.fetch_add(1, Ordering::Relaxed);

                'outer: loop {
                    // Try getting an element from the current thread's vec
                    let current_option_path = pending_paths[thread_index].0.lock().unwrap().pop();

                    match current_option_path {
                        Some(path) => {
                            // We successfully popped -> decrement the len
                            pending_paths[thread_index].1.fetch_sub(1, Ordering::AcqRel);

                            process_path(&pending_paths, &pending, thread_index, &path);
                        }
                        None => {
                            // Try getting one from another thread
                            loop {
                                let mut paths_left = false;

                                let mut to_process = None;

                                // The vecs that could be waited for
                                let mut possible = Vec::new();

                                for (other_index, (_vec, len, waiting)) in
                                    pending_paths.iter().enumerate()
                                {
                                    // Skip checking ourselves
                                    if other_index == thread_index {
                                        continue;
                                    }

                                    // If the other thread's vec isnt empty
                                    if len.load(Ordering::Acquire) != 0 {
                                        paths_left = true;

                                        let waiting = waiting.load(Ordering::Acquire);

                                        if waiting == 0 {
                                            to_process = Some(other_index);
                                            break;
                                        } else {
                                            possible.push((other_index, waiting));
                                        }
                                    }
                                }

                                if to_process.is_none() {
                                    to_process = possible
                                        .iter()
                                        .min_by_key(|(_thread, waiting)| waiting)
                                        .map(|(thread, _waiting)| *thread);
                                }

                                if let Some(other_index) = to_process {
                                    let (vec, len, waiting) = &pending_paths[other_index];

                                    waiting.fetch_add(1, Ordering::AcqRel);
                                    let other_option_path = vec.lock().unwrap().pop();
                                    waiting.fetch_sub(1, Ordering::AcqRel);

                                    if let Some(path) = other_option_path {
                                        // We successfully popped -> decrement the len
                                        len.fetch_sub(1, Ordering::AcqRel);

                                        process_path(&pending_paths, &pending, thread_index, &path);
                                        break;
                                    }
                                }

                                let pending = pending.load(Ordering::Acquire);
                                if pending == 0 && !paths_left {
                                    break 'outer;
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
    pending_paths: &[(Mutex<Vec<PathBuf>>, AtomicUsize, AtomicUsize)],
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
            let (vec, len, _waiting) = &pending_paths[thread_index];
            vec.lock().unwrap().push(path);
            len.fetch_add(1, Ordering::AcqRel);
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
