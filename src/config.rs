use std::{fs, sync::LazyLock};

use crate::util::home;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);

#[derive(Default)]
pub struct Config {
    /// The default subdir of files/
    pub default_subdir: String,
    /// The paths that should be searched by list()
    pub list_paths: Vec<String>,
}
impl Config {
    fn load() -> Self {
        let path = format!("{}/.config/dots", home());

        let string = fs::read_to_string(path).expect("Failed to read config");

        let mut config = Self::default();

        for line in string.lines() {
            let (key, value) = line
                .split_once('=')
                .expect("Config entry should contain a '='");

            match key {
                "default_subdir" => config.default_subdir = value.trim().to_owned(),
                "list_paths" => config
                    .list_paths
                    .extend(value.split(',').map(|value| value.trim().to_string())),
                other => panic!("Unknown config entry: {other}"),
            };
        }

        assert!(
            !config.default_subdir.is_empty(),
            "default_subdir is empty or not in the config"
        );

        config
    }
}
