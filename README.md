# Icicle

[![.github/workflows/build.yml](https://github.com/nishtahir/icicle/actions/workflows/build.yml/badge.svg)](https://github.com/nishtahir/icicle/actions/workflows/build.yml)

A minimalist [OSS CAD Suite](https://github.com/YosysHQ/oss-cad-suite-build) version manager inspired
by [fnm](https://github.com/Schniz/fnm).

## Installation

### Script

For Unix shells, use the automatic [installation script](https://github.com/nishtahir/icicle/blob/main/.ci/install.sh):

```shell
$ curl -fsSL https://raw.githubusercontent.com/nishtahir/icicle/main/.ci/install.sh | bash 
```

To uninstall `icicle` and all installed toolchains, delete the `~/.icicle` directory.

### Manual

For manual installation, download the latest release from the [releases page](). Extract the archive and move the `icicle` binary to a directory in your `PATH`. Then add the following to your shell profile:

```shell
# ~/.bashrc
eval "$(icicle env)"
```

## Commands

```
$ icicle --help

CLI for OSS CAD Suite toolchain management

Usage: icicle [COMMAND]

Commands:
  use          Change the oss cad toolchain version
  default      Set an installed toolchain as the default toolchain
  install      Install an OSS CAD Suite toolchain
  latest       Install the latest OSS CAD Suite toolchain
  uninstall    Uninstall an OSS CAD Suite toolchain
  exec         Run a command with a toolchain's tools, without changing the shell
  which        Print the path of a tool in the active toolchain
  prune        Remove toolchains other than the default and those in use
  completions  Print a shell completion script
  current      Print the active oss cad toolchain version
  env          Print and setup required environment variables for icicle
  list         List installed toolchain versions
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

```

## Usage

* `install` - Installs a new OSS Cad Suite toolchain. You can either provide a version to install or a version in a `.icicle-toolchain` file. The file is looked up in the current directory and then each parent directory, so it applies from anywhere inside a project.

  ```
  // .icicle-toolchain
  2022-07-12

  $ icicle install
  ...

  // or

  $ icicle install 2022-07-12
  ```

* `latest` - Installs the latest released OSS CAD Suite toolchain

  ```
  $ icicle latest
  ```

* `exec` - Run a command with a toolchain's tools on the `PATH`, without changing the shell session. This also works in scripts and CI where `icicle env` was never run. The toolchain is `--using <version>`, otherwise the nearest `.icicle-toolchain` file, otherwise the default. The command's exit code is passed through.

  ```
  $ icicle exec --using 2022-07-12 yosys --version
  $ icicle exec make test
  ```

* `which` - Print the path of a tool in the active toolchain. It looks in the toolchain this shell is using, or in the one given with `--using`

  ```
  $ icicle which yosys
  /Users/me/.icicle/toolchains/2022-07-28/oss-cad-suite/bin/yosys
  ```

* `prune` - Free disk space by removing every toolchain except the default, the one this shell is using, and any that another running shell is using. Leftovers from interrupted installs are removed too. It lists what it will remove and asks before deleting. Use `--dry-run` to only look, or `--yes` to skip the question (required when there is no terminal to ask on)

  ```
  $ icicle prune
  Keeping  2022-07-28 (default, in use by this shell)
  Removing 2022-07-12 (1.9 GB)
  Free 1.9 GB? [y/N] y
  Freed 1.9 GB.
  ```

* `completions` - Print a completion script for `bash`, `zsh`, `fish`, `elvish` or `powershell`

  ```shell
  # ~/.bashrc
  eval "$(icicle completions bash)"

  # ~/.zshrc (after compinit)
  eval "$(icicle completions zsh)"

  # fish
  icicle completions fish > ~/.config/fish/completions/icicle.fish
  ```

* `uninstall` - Uninstalls an OSS Cad Suite toolchain

  ```
  $ icicle uninstall 2022-07-12
  ```

* `list` - List all installed OSS Cad Suite toolchains

  ```
  $ icicle list
  * 2022-07-15 (default)
  * 2022-07-12
  * 2022-07-28
  ```
* `current` - Print the installed OSS CAD Suite toolchain active in the shell session
  ```
  $ icicle current
  2022-07-15
  ```

* `use` - Set an installed version of the OSS CAD Suite toolchain as the current version linked in the shell session. Like `install`, it falls back to the `.icicle-toolchain` file when no version is given
  ```
  $ icicle use 2022-07-28
  $ icicle current
    2022-07-28
  ```

* `default` - Set an installed version of the OSS CAD Suite toolchain as the default version linked with new shell sessions
  ```
  $ icicle default 2022-07-28
    Setting 2022-07-28 as default.
  ```

## Development

```shell
$ cargo test
```

The end to end tests in `tests/cli.rs` run the real binary through `sh` against a throwaway
`ICICLE_HOME`, and download from a local HTTP server instead of GitHub. Set `ICICLE_DIST_MIRROR`
to point `install` at a different mirror of the OSS CAD Suite release assets.

# License

```
Copyright (C) 2022  Nish Tahir

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
```
