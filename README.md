# Icicle

A minimalist [OSS CAD Suite](https://github.com/YosysHQ/oss-cad-suite-build) version manager inspired
by [fnm](https://github.com/Schniz/fnm).

# Usage

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

# Development

## Requirements

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