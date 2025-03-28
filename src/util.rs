use std::{
    env::{self, current_exe},
    fs::{self, File},
    io::{BufReader, Read as _},
    path::{Path, PathBuf},
    process::{Command, exit},
};

use anyhow::{Context as _, Result, anyhow};

use crate::{SILENT, config::CONFIG};

/// The absolute path to the users home directory.
pub fn home() -> Result<String> {
    env::var("HOME").context("Failed to get HOME env variable")
}

pub fn get_hostname() -> Result<String> {
    Ok(fs::read_to_string("/etc/hostname")
        .context("Failed to read /etc/hostname")?
        .trim()
        .into())
}

/// Inform the user of the `failed_action` and rerun with root privileges
#[expect(clippy::expect_used)] // We dont return anyways, so we might as well panic
pub fn rerun_with_root(failed_action: &str) -> ! {
    if !SILENT.get().expect("Failed to get SILENT") {
        println!("{failed_action} requires root privileges",);
    }
    rerun_with_root_args(&[]);
}

/// Rerun with root privileges, and add the provided args to the command
#[expect(clippy::expect_used)] // We dont return anyways, so we might as well panic
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

    let home = home().expect("Failed to get users home di");

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
        .context("Failed to convert path to string")?;

    // Replace {home} with the users home dir
    let resolved_home = str.replace("{home}", &home()?[1..]);

    Ok(if path.is_relative() {
        let index = str
            .find('/')
            .with_context(|| format!("Failed finding '/' in path '{}'", path.display()))?;

        // Only keep the path from the first /
        let absolute = &resolved_home[index..];

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
    let fmt_diff = |difference: &str| {
        Err(anyhow!(
            "Path {} already exists and differs in {difference} to {}",
            system_path.display(),
            config_path.display()
        ))
    };

    // Get metadatas
    let system_metadata = fs::symlink_metadata(system_path).with_context(|| {
        format!(
            "Failed to get metadata for system path {}",
            system_path.display()
        )
    })?;
    let config_metadata = fs::symlink_metadata(config_path).with_context(|| {
        format!(
            "Failed to get metadata for config path {}",
            config_path.display()
        )
    })?;

    if system_metadata.file_type() != config_metadata.file_type() {
        fmt_diff("file type")
    } else if system_metadata.len() != config_metadata.len() {
        fmt_diff("length")
    } else if system_metadata.permissions() != config_metadata.permissions() {
        fmt_diff("permissions")
        // If they are symlinks
    } else if system_metadata.file_type().is_symlink()
    // And their destinations dont match
        && fs::read_link(system_path).with_context(|| format!("reading symlink destination for path {}", system_path.display()))?
        != fs::read_link(config_path).with_context(|| format!("reading symlink destination for path {}", config_path.display()))?
    {
        fmt_diff("symlink destination")
    } else if system_metadata.file_type().is_file() {
        let system_file = File::open(system_path).context("opening system file")?;
        let config_file = File::open(config_path).context("opening config file")?;

        let mut system_reader = BufReader::new(system_file);
        let mut config_reader = BufReader::new(config_file);

        let mut system_buf = [0; 4096];
        let mut config_buf = [0; 4096];

        loop {
            let system_read = system_reader
                .read(&mut system_buf)
                .context("reading system file")?;

            let config_read = config_reader
                .read(&mut config_buf)
                .context("reading config file")?;

            if system_read != config_read {
                fmt_diff("content length")?;
            } else if system_read == 0 {
                return Ok(()); // EOF & identical
            } else if system_buf[..system_read] != config_buf[..config_read] {
                fmt_diff("file contents")?;
            }
        }
    } else {
        Ok(())
    }
}
