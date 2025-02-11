## What
A (dot)file manager using symlinks (intended for use in [atem](https://github.com/jullanggit/atem), but can also be used standalone).

Can be understood as a more powerful GNU Stow, which allows you to precisely control where in the file tree the symlink should be placed.

## How
Dots operates on a file tree containing the paths you want to symlink.
The location of this file tree has to be set using the `files_path` key in the config file (recommendation: `{home}/.config/atem/files`)

The `files_path` directory is split in multiple sub-directories, to allow for different files on different machines.

"{hostname}" can be used as a placeholder for the actual hostname (For example: `{hostname}/etc/pacman.conf`).

As most other symlinks are against the same subdir, you can set a `default_subdir` in the config file.
Then, you can just omit the default subdir. (For example: `/etc/pacman.conf`)

## Commands:
All paths are in the format described above.

- add:     Add the given path to the system
- remove:  Remove the given path from the system (does not remove the files the path points to, only the symlink)
- import:  Import the given path from the system
- list:    Outputs a list of all symlinks on the system that are probably made by dots

All commands (except remove, which doesn't care) can also take --copy as an argument for copying, instead of symlinking the file. This is meant for things that for some reason or another do not like being a symlink.

Note that paths added using --copy will not be detected by list, instead a list of items that should be on the system should be passed, which are then validated. Only paths that actually are on the system are printed back out.

### Import
- Copies the given path from the system into the config, and replaces the system path with a symlink the the config path

### List
- Paths to search for symlinks can be configured in the config file under the `list_paths` key

## Options:
- silent: suppress any non-primary output

## Config file
### Location
`{home}/.config/dots`
### Format
- default_subdir & files_path:
  - key = value
- list_paths
  - key = value(,value,value)
- root
  - True if specified, false otherwise

## Installing
`RUSTFLAGS=-Znext-solver cargo +nightly install --git https://github.com/jullanggit/dots dots`

## Usage
- This software is provided as-is: I make no guarantees that using dots wont fuck up your system, the only testing it currently receives is usage by me.
- Please do report any errors/bugs if you encounter them
