use crate::{
    config::CONFIG,
    util::{
        config_path, get_hostname, paths_equal, rerun_with_root_args,
        rerun_with_root_if_permission_denied, system_path,
    },
};
use std::{
    fs::{self},
    path::Path,
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

    let read_dirs = Mutex::new(Vec::from_iter(
        CONFIG
            .list_paths
            .iter()
            .map(|path| fs::read_dir(path).expect("Failed to read dir")),
    ));

    let pending = AtomicUsize::new(0);

    thread::scope(|scope| {
        let threads = thread::available_parallelism().map_or(12, Into::into);
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let read_dir = read_dirs.lock().unwrap().pop();
                    match read_dir {
                        Some(read_dir) => {
                            pending.fetch_add(1, Ordering::AcqRel);
                            // Ignore errors with .flatten()
                            for dir_entry in read_dir.flatten() {
                                // Get the file type
                                let file_type = dir_entry.file_type().unwrap();

                                if file_type.is_symlink() {
                                    // get the entries target
                                    let target = fs::read_link(dir_entry.path())
                                        .expect("Failed to get target");
                                    // If the target is in the files/ dir...
                                    if let Ok(stripped) = target.strip_prefix(&CONFIG.files_path)
                                        // ...and was plausibly created by dots...
                                        && system_path(stripped) == dir_entry.path()
                                    {
                                        // Convert to a string, so strip_prefix() doesnt remove leading slashes
                                        let str =
                                            stripped.to_str().expect("Item should be valid UTF-8");

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
                                    let read_dir = fs::read_dir(path).expect("Failed to read dir");
                                    read_dirs.lock().unwrap().push(read_dir);
                                }
                            }
                            pending.fetch_sub(1, Ordering::AcqRel);
                        }
                        None => {
                            let pending = pending.load(Ordering::Acquire);
                            if pending == 0 {
                                break;
                            }
                        }
                    }
                }
            });
        }
    });
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
