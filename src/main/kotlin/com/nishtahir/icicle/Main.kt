package com.nishtahir.icicle

import com.github.ajalt.clikt.core.subcommands

fun main(argv: Array<out String>) {
    val environment = Environment.create()

    // Setup CLI
    Cli().subcommands(InstallCommand(environment))
        .subcommands(UninstallCommand(environment))
        .subcommands(ListCommand(environment))
        .subcommands(DefaultCommand(environment))
        .subcommands(EnvCommand(environment))
        .main(argv)
}
