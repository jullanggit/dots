use std::{
    env::{self, current_exe},
    fs::{self, File},
    io::{self, BufReader, ErrorKind, Read as _},
    path::{Path, PathBuf},
    process::{Command, exit},
};

use color_eyre::eyre::{Context as _, OptionExt as _, Result, eyre};

use crate::{SILENT, config::CONFIG};

/// The absolute path to the users home directory.
pub fn home() -> Result<String> {
    env::var("HOME").wrap_err("Failed to get HOME env variable")
}

pub fn get_hostname() -> Result<String> {
    Ok(fs::read_to_string("/etc/hostname")
        .wrap_err("Failed to read /etc/hostname")?
        .trim()
        .into())
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
        .and_then(|path| path.to_str().map(ToOwned::to_owned))
    {
        args[0] = absolute_path;
    }

    let home = home().unwrap();

    let status = Command::new("/usr/bin/sudo")
        // Preserve $HOME
        .arg(format!("HOME={home}"))
        .args(args)
        .spawn()
        .expect("Failed to spawn child process")
        .wait()
        .expect("Failed to wait on child process");

    if status.success() {
        exit(0);
    } else {
        exit(status.code().unwrap_or(1));
    }
}

/// Converts the path relative to files/ to the location on the actual system. (by trimming the subdir of files/ away)
pub fn system_path(path: &Path) -> Result<PathBuf> {
    let str = path
        .as_os_str()
        .to_str()
        .ok_or_eyre("Failed to convert path to string")?;

    // Replace {home} with the users home dir
    let resolved_home = str.replace("{home}", &home()?[1..]);

    Ok(if path.is_relative() {
        // Only keep the path from the first /
        let absolute = &resolved_home[str
            .find('/')
            .ok_or_else(|| eyre!("Failed finding '/' in path '{}'", path.display()))?..];

        absolute.into()
    } else {
        // The default subdir was elided, so the path is already the correct one
        resolved_home.into()
    })
}

/// Converts the path that should be symlinked to the path in the files/ directory
#[expect(clippy::literal_string_with_formatting_args)]
pub fn config_path(mut cli_path: &Path) -> Result<PathBuf> {
    assert!(
        !Path::new(&CONFIG.default_subdir).is_absolute(),
        "Default subdir is not allowed to be absolute"
    );

    let mut config_path = PathBuf::from(&CONFIG.files_path);

    // If the path started with "/", the default subdir was elided
    if let Ok(relative_path) = cli_path.strip_prefix("/") {
        // So we add it
        config_path.push(&CONFIG.default_subdir);

        // And replace the absolute path with the relative one to avoid overwriting the entire config_path
        cli_path = relative_path;
    }
    // If the default subdir wasn't elided, replace "{hostname}" with the actual hostname
    else if let Ok(stripped_path) = cli_path.strip_prefix("{hostname}") {
        let hostname = get_hostname();
        config_path.push(hostname?.trim());

        cli_path = stripped_path;
    }

    // Replace "{home}" with the users home dir
    if let Ok(stripped_path) = cli_path.strip_prefix("{home}") {
        let home = home()?;
        config_path.push(&home[1..]); // skip the leading '/' to avoid overwriting the entire config_path

        cli_path = stripped_path;
    }

    config_path.push(cli_path);

    Ok(config_path)
}

/// Checks if the config & system paths are already equal
/// Does *not* currently support directories
#[expect(clippy::filetype_is_file)]
pub fn paths_equal(config_path: &Path, system_path: &Path) -> Result<()> {
    // Get metadatas
    let system_metadata = rerun_with_root_if_permission_denied(
        fs::symlink_metadata(system_path),
        "getting metadata for system path",
    )?;
    let config_metadata = rerun_with_root_if_permission_denied(
        fs::symlink_metadata(config_path),
        "getting metadata for config path",
    )?;

    if system_metadata.file_type() != config_metadata.file_type() {
        Err(eyre!("Path already exists and differs in file type"))
    } else if system_metadata.len() != config_metadata.len() {
        Err(eyre!("Path already exists and differs in len"))
    } else if system_metadata.permissions() != config_metadata.permissions() {
        Err(eyre!("Path already exists and differs in permissions"))
        // If they are symlinks
    } else if system_metadata.file_type().is_symlink()
    // And their destinations dont match
        && rerun_with_root_if_permission_denied(
            fs::read_link(system_path),
            "reading symlink destination for system path",
        )? != rerun_with_root_if_permission_denied(
            fs::read_link(system_path),
            "reading symlink destination for system path",
        )?
    {
        Err(eyre!(
            "Path already exists and differs in symlink destination"
        ))
    } else if system_metadata.file_type().is_file() {
        let system_file =
            rerun_with_root_if_permission_denied(File::open(system_path), "opening system file")?;
        let config_file =
            rerun_with_root_if_permission_denied(File::open(config_path), "opening config file")?;

        let mut system_reader = BufReader::new(system_file);
        let mut config_reader = BufReader::new(config_file);

        let mut system_buf = [0; 4096];
        let mut config_buf = [0; 4096];

        loop {
            let system_read = rerun_with_root_if_permission_denied(
                system_reader.read(&mut system_buf),
                "reading system file",
            )?;
            let config_read = rerun_with_root_if_permission_denied(
                config_reader.read(&mut config_buf),
                "reading config file",
            )?;

            if system_read != config_read {
                return Err(eyre!("Path already exists and differs in content length"));
            } else if system_read == 0 {
                return Ok(()); // EOF & identical
            } else if system_buf[..system_read] != config_buf[..config_read] {
                return Err(eyre!("Path already exists and differs in file contents"));
            }
        }
    } else {
        Ok(())
    }
}

/// Inform the user of the `failed_action` and rerun with root privileges, if the result is a `PermissionDenied`, panic on any other error
pub fn rerun_with_root_if_permission_denied<T>(result: io::Result<T>, action: &str) -> Result<T> {
    Ok(result.inspect_err(|e| {
        if e.kind() == ErrorKind::PermissionDenied {
            rerun_with_root(action)
        }
    })?)
}
