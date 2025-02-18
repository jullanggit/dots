use std::{fs, sync::LazyLock};

use crate::util::home;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);

#[derive(Default)]
pub struct Config {
    /// The default subdir of files/
    pub default_subdir: String,
    /// The path to the files/ directory
    pub files_path: String,
    /// The paths that should be searched by `list()`
    pub list_paths: Vec<String>,
    /// Whether to run 'list' with root privileges
    pub root: bool,
}
impl Config {
    fn load() -> Self {
        let path = format!("{}/.config/dots", home());

        let string = fs::read_to_string(path).expect("Failed to read config");

        let mut config = Self::default();

        for line in string.lines() {
            match line.split_once('=') {
                Some((key, value)) => match key.trim() {
                    "default_subdir" => value.trim().clone_into(&mut config.default_subdir),
                    "files_path" => value.trim().clone_into(&mut config.files_path),
                    "list_paths" => config
                        .list_paths
                        .extend(value.split(',').map(|value| value.trim().to_owned())),
                    "root" => config.root = true,
                    other => panic!("Unknown config entry: {other}"),
                },
                None => match line.trim() {
                    "root" => config.root = true,
                    other => panic!("Unknown config key: {other}"),
                },
            }
        }

        assert!(
            !config.default_subdir.is_empty(),
            "default_subdir is empty or not in the config"
        );

        config
    }
}
