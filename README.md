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

"{home}" can be used as a placeholder for the home directory of the current user (For example: `/{home}/.config/dots`).

## Ideas, contributing, bugs etc
- Dots is still very much under development, so if you have any ideas / feature requests or encounter any bugs, please open an issue or a PR

## Commands
All paths are in the format described above.

- add:     Add the given path to the system
- remove:  Remove the given path from the system (does not remove the files the path points to, only the symlink)
- import:  Import the given path from the system
> [!WARNING]
Import has sometimes eaten data and I haven't yet had the time to track it down, so make sure to have a backup, or import manually (or of course fix the bug :D)
- list:    Outputs a list of all symlinks on the system that are probably made by dots
- config:  Interactively creates the config file

All commands (except remove, which doesn't care) can also take --copy as an argument for copying, instead of symlinking the file. This is meant for things that for some reason or another do not like being a symlink.

Note that paths added using --copy will not be detected by list, instead a list of items that should be on the system should be passed, which are then validated. Only paths that actually are on the system are printed back out.

### Import
- Copies the given path from the system into the config, and replaces the system path with a symlink the the config path

### List
- Paths to search for symlinks can be configured in the config file under the `list_paths` key

## Options
- silent: suppress any non-primary output

## Config file
### Location
`{home}/.config/dots`
### Format
- default_subdir & files_path:
  - key = value
- list_paths & ignore_paths
  - key = value(,value,value)
- root
  - True if specified, false otherwise

### files_path
- the path to the files/ directory
### default_subdir
- the default subdir that will get filled in when subdir is elided
### list_paths
- the paths that `list` searches through
### ignore_paths
- the paths that `list` ignores
### root
- whether `list` should run as root

## Installing
`cargo +nightly install atem-dots`

## Usage
- This software is provided as-is: I make no guarantees that using dots wont fuck up your system, the only testing it currently receives is usage by me and a few other users.
- Please do report any errors/bugs if you encounter them
 
![brainmademark](https://brainmade.org/black-logo.svg)
