# Changelog

All notable changes to this project will be documented in this file.

## 0.2.1

### Cargo.toml
#### categories

- config -> filesystem

## 0.2.0

### Feature

- add Debug commands & config-path debug command

- replace "{home}" with the users home dir in paths

### Other

- Release

- Add initial ARCHITECTURE.md

### fix
#### command_descriptions

- Add missing "."'s

### git-cliff

- sort commits from newest to oldest

- change sort_commits to oldest

## 0.1.3

### Other

- Release

- remove unneeded .cargo/config.toml

### code
#### list

- improve work stealing and extract it into function

- some readability improvements

- extract len & waiting handeling into a struct

### list

- make threads choose the thread with the least waiting threads to steal work from

- attempt to mitigate lock contention by manually sharding vecs

## 0.1.2

### Other

- Release

### README

- update installing section

### cargo-release

- add release.toml

## 0.1.1

### CHANGELOG

- initial changelog

### Other

- Release atem-dots version 0.1.1

### TODO

- remove 'parallel list feature flag'

- add 'parallel list feature flag'

### git-cliff

- add cliff.toml

### list

- dont unnecessarily hold dir fd's

- use as many threads as cores

- thread::spawn() for every read_dir -> thread pool

- tokio -> manual thread::spawn for read_dir()

## 0.1.0

### Cargo.toml

- change repo back to dots

### Other

- remove unused macros feature from tokio

- add LICENSE

- rename package (but not binary) to atem-dots

- Add rust-toolchain file

- meta -> atem

- convert all possible manual handelings of permissiondenied to rerun_with_root_if_permission_denied

- add rerun_with_root_if_permission denied and use it where ever possible

- fix arg parsing for list --copy

- Add --copy for copying files instead of symlinking them

- add install command to README

- add -Znext-solver to the rustflags and mention building with nightly in the README

- make list() multithreaded

- start making list() async

- make list() single-threaded again, because it is faster and way simpler
Use a simple VecDeque, instead of channels

- fix running list() with root

- add option to run list() with root

- Make list() parallel agian by using workqueue channel (also remove dependency on walkdir & rayon)

- start manually implementing WalkDir

- handle SILENT-setting failure

- make --silent a global flag (and turn it into a static OnceLock)

- add silent flag to list

- make list rerun with root if one of the root paths returns a PermissionDenied

- Add some TODOs

- Handle copying directories with import

- Rerun with root for removing path

- small fixes

- update README.md

- add command descriptions for add & remove

- Make the files_path configurable

- Update README.md

- add config

- add import command (and add force flag to add)

- move rerun_with_root(), system_path() & config_path() into util

- error_with_message -> panic!

- split stuff into different files

- first check if the entry is a symlink before trying to read it (way faster)

- Also check if the found symlinks were plausibly created by dots

- format items before printing them in list()

- make list() parallel using rayon

- implement list()

- Update README

- remove file before overwriting

- add missing .trim() after getting hostname

- fix system_path()

- add missing / in config_path

- get hostname from /etc/hostname

- Also display where we are symlinking to

- extend README, TODO and files.toml

- allow using {hostname}

- Extend TODO and README

- Actually use backticks in comment

- Remove dependency on sudo

- Add retry_with_root()

- make create_symlink() automatically create parent dirs

- Make add() ask for overwrite instead of retry

- If system_path is already a symlink, also check if it points to the correct location

- Use the sudo crate for automatically rerunning as root

- add error_with_message()

- Add to TODO

- Add description to add() and improve its variable names

- Improve the code in config_path (comments & better control-flow)

- Handle io errors in bool_question

- Change check for default_subdir being absolute to actually use .is_absolute()

- Rename trim_files_subdir() to system_path(), improve its readablility, and make it return a &Path

- Rename get_origin() to config_path()

- Remove TrimFilesSubdir command

- Add TODO

- Rebrand to dots

- Fix absolute path overriding the other parts of the path

- Fix the home path being overriden by the rebos files path

- use HOME env variable instead of ~

- slight reformatting of the readme

- slight rewording

- add example managers/files.toml

- Add README.md

- some cli improvements + ability to ommit the default sub-dir of files (common by default)

- initial commit

### README

- add documentation for --copy

- add no-guarantee section

### code

- std::process::exit(1) -> exit(1)

### list

- fix printing condition with --copy

- add some more comments

- print items inline, instead of collecting them first

- use a semaphore again

- Add comments

### list --copy

- first check if the system path exists before checking if it is equal

<!-- generated by git-cliff -->
