package com.nishtahir.icicle

import com.github.ajalt.clikt.core.CliktCommand
import com.github.ajalt.clikt.parameters.arguments.argument
import com.github.ajalt.clikt.parameters.options.flag
import com.github.ajalt.clikt.parameters.options.option
import kotlin.system.exitProcess

class Cli : CliktCommand() {
    private val verbose: Boolean by option("-v", "--verbose", help = "Display verbose output")
        .flag()

    override fun run() {

    }
}

class Install(
    private val manifest: Manifest,
    private val toolchainDownloader: ToolchainDownloader
) : CliktCommand() {
    override fun run() {
        toolchainDownloader.downloadAndExtractToolchain(manifest.toolchain)
    }
}

class Run(private val manifest: Manifest) : CliktCommand() {
    private val script by argument(help = "Script command to run")
    override fun run() {
        val command = manifest.scripts[script]
        if (command == null) {
            val availableScripts = manifest.scripts.keys.sorted()
            println("Unknown script '$script'.")
            println("Available scripts are ${availableScripts.joinToString { "'$it'" }}")
            exitProcess(1)
        }
        ScriptExecutor(manifest).executeScript(script)
    }
}
