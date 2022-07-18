package com.nishtahir.icicle

import com.github.ajalt.clikt.core.CliktCommand
import com.github.ajalt.clikt.parameters.arguments.argument
import com.github.ajalt.clikt.parameters.arguments.optional
import com.github.ajalt.clikt.parameters.options.flag
import com.github.ajalt.clikt.parameters.options.option
import java.io.File
import java.nio.file.Files
import kotlin.system.exitProcess

class Cli : CliktCommand(name = "icicle") {
    private val verbose: Boolean by option("-v", "--verbose", help = "Display verbose output")
        .flag()

    override fun run() {

    }
}

class InstallCommand(
    private val manifest: Manifest,
    private val toolchainDownloader: ToolchainDownloader
) : CliktCommand(help = "Install an OSS CAD Suite toolchain") {
    private val version by argument(help = "Toolchain version to install").optional()

    override fun run() {
        val toolchainVersion = version
            ?: manifest.toolchain
            ?: throw IllegalStateException("Please provide a toolchain to install or declare a toolchain in an icicle.yml file.")
        toolchainDownloader.downloadAndExtractToolchain(toolchainVersion)
    }
}

class UninstallCommand(private val manifest: Manifest) : CliktCommand(help = "Uninstall an OSS CAD Suite toolchain") {
    private val version by argument(help = "Toolchain version to uninstall")

    override fun run() {
        val uninstallPath = File("${manifest.toolchainHome}/$version")
        if (!uninstallPath.isDirectory) {
            println("'$version' is not a valid toolchain directory.")
            exitProcess(1)
        }
        println("Uninstalling '$uninstallPath'")
        uninstallPath.deleteRecursively()
    }
}

class ListCommand(private val manifest: Manifest) : CliktCommand(help = "List installed toolchain versions") {
    override fun run() {
        // Check for default
        val defaultAlias = File("${manifest.aliasesHome}/default").toPath().toRealPath()

        // Check for installed toolchains
        File(manifest.toolchainHome)
            .listFiles(File::isDirectory)
            ?.forEach {
                var line = "* ${it.nameWithoutExtension}"
                if (it.toPath() == defaultAlias) {
                    line += " default"
                }
                println(line)
            }
    }
}

class DefaultCommand(private val manifest: Manifest) :
    CliktCommand(help = "Set an installed toolchain as the default toolchain") {
    private val version by argument(help = "Toolchain version to make default")

    override fun run() {
        val aliasesHome = File(manifest.aliasesHome)
        if (!aliasesHome.exists()) {
            aliasesHome.mkdirs()
        }

        val toolchainPath = File("${manifest.toolchainHome}/$version")
        if (!toolchainPath.isDirectory) {
            println("'$version' is not a valid toolchain directory.")
            exitProcess(1)
        }
        val defaultAlias = File("${manifest.aliasesHome}/default")
        if (defaultAlias.exists()) {
            defaultAlias.delete()
        }
        println("Setting $version as default.")
        Files.createSymbolicLink(defaultAlias.toPath(), toolchainPath.toPath())
    }
}

class RunCommand(private val manifest: Manifest) : CliktCommand(help = "Run an arbitrary icicle script") {
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

class EnvCommand(private val manifest: Manifest) : CliktCommand(help = "") {
    override fun run() {
        val defaultAlias = File("${manifest.aliasesHome}/default/").toPath()
        val binPath = "${defaultAlias}/oss-cad-suite/bin"
        val libExecPath = "${defaultAlias}/oss-cad-suite/libexec"
        println("export PATH=$binPath:PATH")
        println("export PATH=$libExecPath:PATH")
    }
}