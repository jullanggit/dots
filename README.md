A (dot)file manager using symlinks (intended for use in rebos/meta)

In combination with something like rebos/meta it is basically a more powerful GNU Stow,
which allows you to precisely contoll where in the file tree the symlink should be placed

Apart from cleanup, the user shouldnt be required to manually interact with any files in the file tree

Intended File-structure:
```
.config/rebos
├── files
    ├── other
│   └── common
├── gen.toml
├── imports
│   └── ...
├── machines
│   └── ...
├── manager_order.toml
└── managers
    ├── ...
    └── files.toml
```

As you sometimes want files to differ on different machines, the 'files' directory is split in multiple sub-directories.
{hostname} can be used as a placeholder for the actual hostname

Intended usage: `{some sub-directory of files}/{the file path you want to symlink}` (For example: `common/etc/pacman.conf`)

Because most of the time symlinks are against the common directory, you can just omit the "common". (For example: `/etc/pacman.conf`) (The default sub-dir can be configured with `--default-sub-dir`)
Because most other symlinks are against the current hostname, "{hostname}" can be used as a placeholder. (For example: '{hostname}/etc/pacman.conf')
