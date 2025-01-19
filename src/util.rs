use std::{env, fs};

/// The path of the files/ directory
pub fn files_path() -> String {
    format!("{}/.config/rebos/files", home())
}

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
