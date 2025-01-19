## What
A (dot)file manager using symlinks (intended for use in [meta](https://github.com/jullanggit/meta)).

Can be understood as a more powerful GNU Stow, which allows you to precisely control where in the file tree the symlink should be placed.

## How
Dots operates on a file tree containing the paths you want to symlink.
The location of this file tree has to be set using the `files_path` key in the config file (recommendation: `{home}/.config/meta/files`)

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

### Import
- Copies the given path from the system into the config, and replaces the system path with a symlink the the config path

### List
- Paths to search for symlinks can be configured in the config file under the `list_paths` key

## Config file
### Location
`{home}/.config/dots`
### Format
key = value(,value,value)
