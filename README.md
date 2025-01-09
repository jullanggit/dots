A (dot)file manager using symlinks (intended for use in [meta](https://github.com/jullanggit/meta))

Can be understood as a more powerful GNU Stow, which allows you to precisely control where in the file tree the symlink should be placed

Apart from cleanup, the user shouldn't be required to manually interact with any files in the file tree

Intended File-structure:
```
.config/meta
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
{hostname} can be used as a placeholder for the actual hostname (For example: `{hostname}/etc/pacman.conf`)

Intended path format: `{sub-directory of files}/{the file path you want to symlink}` (For example: `xyz/etc/pacman.conf`)

As most of the time symlinks are against the "common"" directory, you can just omit the "common". (For example: `/etc/pacman.conf`) (The default sub-dir can be configured with `--default-sub-dir`)
