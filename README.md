# Icicle

[![.github/workflows/build.yml](https://github.com/nishtahir/icicle/actions/workflows/build.yml/badge.svg)](https://github.com/nishtahir/icicle/actions/workflows/build.yml)

A minimalist [OSS CAD Suite](https://github.com/YosysHQ/oss-cad-suite-build) version manager inspired
by [fnm](https://github.com/Schniz/fnm).

## Installation

For Unix shells, use the automatic [installation script](https://github.com/nishtahir/icicle/blob/main/.ci/install.sh):

```shell
$ curl -fsSL https://raw.githubusercontent.com/nishtahir/icicle/main/.ci/install.sh | bash 
```

To uninstall `icicle` and all installed toolchains simply delete the `~/.icicle` directory.

## Commands

```
$ icicle --help

Usage: icicle [OPTIONS] COMMAND [ARGS]...

Options:
  --version   Show the version and exit
  -h, --help  Show this message and exit

Commands:
  default    Set an installed toolchain as the default toolchain
  current    Print the active oss cad toolchain version
  env        Print and setup required environment variables for icicle
  install    Install an OSS CAD Suite toolchain
  list       List installed toolchain versions
  uninstall  Uninstall an OSS CAD Suite toolchain
  use        Change the oss cad toolchain version

```

## Usage

* `install` - Installs a new OSS Cad Suite toolchain. You can either provide a version to install or a version in a `.icicle-toolchain` file.

  ```
  // .icicle-toolchain
  2022-07-12

  $ icicle install
  Downloading: https://github.com/YosysHQ/oss-cad-suite-build/releases/download/2022-07-12/oss-cad-suite-darwin-arm64-20220712.tgz
  ...

  // or

  $ icicle install 2022-07-12
  ```

* `list` - List all installed OSS Cad Suite toolchains

  ```
  $ icicle list
  * 2022-07-15 default
  * 2022-07-12
  * 2022-07-28
  ```
* `current` - Print the installed OSS CAD Suite toolchain active in the shell session
  ```
  $ icicle current
  2022-07-15
  ```

* `use` - Set an installed version of the OSS CAD Suite toolchain as the current version linked in the shell session
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

### Requirements

* JDK 11
* GraalVM

You will need to enable `native-image` builds. See the
[GraalVM setup](https://graalvm.github.io/native-build-tools/0.9.4/graalvm-setup.html) setup
documentation for more information.

## Building

To build the native image for your platform.

```shell
$ ./gradlew nativeCompile
```

This will output the compiled native binary to `./build/native/nativeCompile/icicle`

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
