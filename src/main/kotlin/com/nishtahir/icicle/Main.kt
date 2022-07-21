package com.nishtahir.icicle

import com.github.ajalt.clikt.core.subcommands

fun main(argv: Array<out String>) {
    val environment = Environment.create()

    // Setup CLI
    Cli()
        .subcommands(DefaultCommand(environment))
        .subcommands(CurrentCommand(environment))
        .subcommands(EnvCommand(environment))
        .subcommands(InstallCommand(environment))
        .subcommands(ListCommand(environment))
        .subcommands(UninstallCommand(environment))
        .subcommands(UseCommand(environment))
        .main(argv)
}
