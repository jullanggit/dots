use std::{
    env::{self, current_exe},
    fs,
    path::{Path, PathBuf},
    process::{exit, Command},
};

use crate::config::CONFIG;

/// The users home directory
pub fn home() -> String {
    env::var("HOME").expect("HOME env variable not set")
}

pub fn get_hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .expect("Failed to get hostname")
        .trim()
        .into()
}

/// Inform the user of the `failed_action` and rerun with root privileges
pub fn rerun_with_root(failed_action: &str) -> ! {
    println!("{failed_action} requires root privileges",);

    // Collect args
    let mut args: Vec<_> = env::args().collect();

    // Overwrite the exe path with the absolute path if possible
    if let Some(absolute_path) = current_exe()
        .ok()
        .and_then(|path| path.to_str().map(|path| path.to_owned()))
    {
        args[0] = absolute_path;
    }

    let home = env::var("HOME").expect("HOME env variable not set");

    let status = Command::new("/usr/bin/sudo")
        // Preserve $HOME
        .arg(format!("HOME={home}"))
        .args(args)
        .spawn()
        .expect("Failed to spawn child process")
        .wait()
        .expect("Failed to wait on child process");

    if !status.success() {
        exit(status.code().unwrap_or(1));
    } else {
        exit(0);
    }
}

/// Converts the path relative to files/ to the location on the actual system. (by trimming the subdir of files/ away)
pub fn system_path(path: &Path) -> &Path {
    if path.is_relative() {
        let str = path.as_os_str().to_str().unwrap();

        // Only keep the path from the first /
        Path::new(&str[str.find('/').unwrap()..])
    } else {
        // The default subdir was elided, so the path is already the correct one
        path
    }
}

/// Converts the path that should be symlinked to the path in the files/ directory
#[expect(clippy::literal_string_with_formatting_args)]
pub fn config_path(mut cli_path: &Path) -> PathBuf {
    if Path::new(&CONFIG.default_subdir).is_absolute() {
        panic!("Default subdir is not allowed to be absolute");
    }

    let mut config_path = PathBuf::from(&CONFIG.files_path);

    // If the path started with "/", the default subdir was elided
    if let Ok(relative_path) = cli_path.strip_prefix("/") {
        // So we add it
        config_path.push(&CONFIG.default_subdir);

        // And replace the absolute path with the relative one to avoid overwriting the entire config_path
        cli_path = relative_path
    }

    // Replace "{hostname}" with the actual hostname
    if let Ok(stripped_path) = cli_path.strip_prefix("{hostname}") {
        let hostname = get_hostname();
        config_path.push(hostname.trim());

        cli_path = stripped_path;
    }

    config_path.push(cli_path);

    config_path
}
