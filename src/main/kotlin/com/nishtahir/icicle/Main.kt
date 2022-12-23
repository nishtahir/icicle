package com.nishtahir.icicle

import com.github.ajalt.clikt.core.subcommands
import com.github.ajalt.clikt.parameters.options.versionOption

fun main(argv: Array<out String>) {
    val s: String = 5

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
        .versionOption(environment.version)
        .main(argv)
}
