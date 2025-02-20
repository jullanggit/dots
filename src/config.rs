use std::{fs, path::PathBuf, sync::LazyLock};

use anyhow::{Context as _, Result, bail, ensure};

use crate::util::home;

#[expect(clippy::unwrap_used)]
pub static CONFIG: LazyLock<Config> = LazyLock::new(|| Config::load().unwrap());

#[derive(Default)]
pub struct Config {
    /// The default subdir of files/
    pub default_subdir: String,
    /// The path to the files/ directory
    pub files_path: String,
    /// The paths that should be searched by `list()`
    pub list_paths: Vec<String>,
    /// The paths that shouldn't be searched by `list()`
    pub ignore_paths: Vec<PathBuf>,
    /// Whether to run 'list' with root privileges
    pub root: bool,
}
impl Config {
    fn load() -> Result<Self> {
        let path = format!("{}/.config/dots", home()?);

        let string = fs::read_to_string(path)
            .context("Failed to read config. Maybe try creating {home}/.config/dots")?;

        let mut config = Self::default();

        for line in string.lines() {
            match line.split_once('=') {
                Some((key, value)) => match key.trim() {
                    "default_subdir" => value.trim().clone_into(&mut config.default_subdir),
                    "files_path" => value.trim().clone_into(&mut config.files_path),
                    "list_paths" => config
                        .list_paths
                        .extend(value.split(',').map(|value| value.trim().to_owned())),
                    "ignore_paths" => config
                        .ignore_paths
                        .extend(value.split(',').map(|value| value.trim().into())),
                    "root" => config.root = true,
                    other => bail!("Unknown config entry: {other}"),
                },
                None => match line.trim() {
                    "root" => config.root = true,
                    other => bail!("Unknown config key: {other}"),
                },
            }
        }

        ensure!(
            !config.default_subdir.is_empty(),
            "default_subdir is empty or not in the config. Maybe try adding something like `default_subdir = common` to your dots config file"
        );

        Ok(config)
    }
}
