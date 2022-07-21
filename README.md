# Icicle

A minimalist [OSS CAD Suite](https://github.com/YosysHQ/oss-cad-suite-build) version manager inspired
by [fnm](https://github.com/Schniz/fnm).

# Usage

## Installation

TODO 

## Commands

```
$ icicle --help

Usage: icicle [OPTIONS] COMMAND [ARGS]...

Options:
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
$ ./gradlew nativeBuild
```

This will output the compiled native binary to `./build/native/nativeBuild/icicle`

