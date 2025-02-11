use std::{
    env::{self, current_exe},
    fs::{self, File},
    io::{BufReader, Read as _},
    path::{Path, PathBuf},
    process::{Command, exit},
};

use crate::{SILENT, config::CONFIG};

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
    if !SILENT.get().unwrap() {
        println!("{failed_action} requires root privileges",);
    }
    rerun_with_root_args(&[]);
}

/// Rerun with root privileges, and add the provided args to the command
pub fn rerun_with_root_args(args: &[&str]) -> ! {
    // Collect args
    let mut args: Vec<_> = env::args()
        .chain(args.iter().map(|&str| str.to_owned()))
        .collect();

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

/// Checks if the config & system paths are already equal
/// Does *not* currently support directories
pub fn paths_equal(config_path: &Path, system_path: &Path) -> Result<(), &'static str> {
    // Get metadatas
    let system_metadata = fs::symlink_metadata(system_path).unwrap(); // TODO: handle permissionedenied
    let config_metadata = fs::symlink_metadata(config_path).unwrap(); // TODO: handle permissionedenied

    if system_metadata.file_type() != config_metadata.file_type() {
        Err("Path already exists and differs in file type")
    } else if system_metadata.len() != config_metadata.len() {
        Err("Path already exists and differs in len")
    } else if system_metadata.permissions() != config_metadata.permissions() {
        Err("Path already exists and differs in permissions")
    } else if system_metadata.file_type().is_symlink()
    // TODO: handle PermissionDenied
        && fs::read_link(system_path).unwrap() != fs::read_link(system_path).unwrap()
    {
        Err("Path already exists and differs in symlink destination")
    } else if system_metadata.file_type().is_file() {
        let system_file = File::open(system_path).unwrap(); // TODO: handle PermissionDenied
        let config_file = File::open(config_path).unwrap(); // TODO: handle PermissionDenied

        let mut system_reader = BufReader::new(system_file);
        let mut config_reader = BufReader::new(config_file);

        let mut system_buf = [0; 4096];
        let mut config_buf = [0; 4096];

        loop {
            let system_read = system_reader.read(&mut system_buf).unwrap(); // TODO: handle PermissionDenied
            let config_read = config_reader.read(&mut config_buf).unwrap(); // TODO: handle PermissionDenied

            if system_read != config_read {
                return Err("Path already exists and differs in content length");
            } else if system_read == 0 {
                return Ok(()); // EOF & identical
            } else if system_buf[..system_read] != config_buf[..config_read] {
                return Err("Path already exists and differs in file contents");
            }
        }
    } else {
        Ok(())
    }
}
