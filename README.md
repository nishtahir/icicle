# Icicle
A minimalist build tool to manage and execute commands using [OSS CAD Suite](https://github.com/YosysHQ/oss-cad-suite-build).

# Usage

```shell
$ icicle --help

Usage: icicle [OPTIONS] COMMAND [ARGS]...

Options:
  -v, --verbose  Display verbose output
  -h, --help     Show this message and exit

Commands:
  install    Install an OSS CAD Suite toolchain
  uninstall  Uninstall an OSS CAD Suite toolchain
  list       List installed toolchain versions
  default    Set an installed toolchain as the default toolchain
  run        Run an arbitrary icicle script
  env        Print required environment variables for icicle

```

Add a `icicle.yml` manifest to your project. This should include a `toolchain` as well as 
any number of scripts you wish to execute later. The `toolchain` should correspond to an OSS CAD Suite release.

```yml
# icicle.yml
name: sample
version: 1.0.0
toolchain: 2022-07-12
scripts:
  sys: yosys -p 'synth_ice40 -top top -json build/alhambra.json' source/*.v
  constraint: cat constraint/*.pcf > build/all.pcf
  pnr: nextpnr-ice40 --hx8k --package tq144:4k --json build/alhambra.json  --pcf
    build/all.pcf  --asc build/alhambra.asc
  pack: icepack build/alhambra.asc build/alhambra.bin
  prog: iceprog build/alhambra.bin

```

Running the `install` command will download the corresponding OSS CAD Suite version declared in the manifest.
``` bash
$ icicle install
```

Scripts are declared as simple bash scripts that will be executed within the OSS CAD Suite executable environment.
Scripts can be executed using the `run` command.

```bash
$ icicle run [name]
```
