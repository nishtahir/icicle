package com.nishtahir.icicle

import com.github.ajalt.clikt.core.CliktCommand
import com.github.ajalt.clikt.parameters.arguments.argument
import com.github.ajalt.clikt.parameters.arguments.optional
import com.github.kittinunf.fuel.Fuel
import org.apache.commons.compress.archivers.tar.TarArchiveInputStream
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream
import java.io.BufferedInputStream
import java.io.File
import java.net.URI
import java.nio.file.Files
import java.nio.file.Path
import kotlin.math.roundToInt
import kotlin.system.exitProcess


class Cli : CliktCommand(name = "icicle", printHelpOnEmptyArgs = true) {
    override fun run() {

    }
}

class InstallCommand(private val env: Environment) : CliktCommand(help = "Install an OSS CAD Suite toolchain") {
    private val version by argument(help = "Toolchain version to install").optional()

    override fun run() {
        val toolchainVersion = version
            ?: env.toolchainFileVersion()
            ?: throw IllegalStateException("Please provide a toolchain to install or declare a toolchain in a '${Environment.TOOLCHAIN_FILE_NAME}' file.")
        downloadAndExtractToolchain(toolchainVersion)
    }


    /**
     * Versions are provided as dates
     */
    private fun downloadAndExtractToolchain(version: String) {
        val temp = downloadToolchainToTemp(version)
        extractTempFileToToolchainDir(temp, version)
    }

    private fun downloadToolchainToTemp(version: String): File {
        val url = createUrlWithToolchainVersion(version)
        val tempFileName = File(URI(url).path).nameWithoutExtension
        return File.createTempFile(tempFileName, "tgz")
            .also { temp -> downloadFile(url, temp) }
    }

    private fun downloadFile(url: String, temp: File) {
        println("Downloading: $url")
        Fuel.download(url)
            .fileDestination { _, _ -> temp }
            .progress(::renderProgress)
            .response()
    }

    private fun renderProgress(readBytes: Long, totalBytes: Long) {
        val progress = ((readBytes.toFloat() / totalBytes.toFloat()) * 10).roundToInt()
        print("${".".repeat(progress)}\r")
    }

    private fun createUrlWithToolchainVersion(toolchain: String): String {
        val minified = toolchain
            .replace("-", "")

        return OSS_TOOLS_URL_TEMPLATE
            .replace("{{date}}", toolchain)
            .replace("{{minified-date}}", minified)
            .replace("{{os}}", getOsName())
            .replace("{{arch}}", getArchName())
    }


    private fun extractTempFileToToolchainDir(temp: File, toolchain: String) {
        val toolchainHome = File(env.toolchainHome)
        if (!toolchainHome.exists()) {
            toolchainHome.mkdirs()
        }

        val toolchainDir = File("${env.toolchainHome}/$toolchain")
        if (toolchainDir.exists()) {
            toolchainDir.deleteRecursively()
        }

        println("Extracting to $toolchainDir")
        Files.newInputStream(temp.toPath())
            .useWith(::BufferedInputStream)
            .useWith(::GzipCompressorInputStream)
            .useWith(::TarArchiveInputStream)
            .use { tarArchiveInputStream ->
                while (true) {
                    val entry = tarArchiveInputStream.nextEntry ?: break
                    val file = File("$toolchainDir/${entry.name}")
                    if (entry.isDirectory) {
                        if (!file.exists()) {
                            file.mkdirs()
                        }
                    } else {
                        file.parentFile.mkdirs()
                        Files.copy(tarArchiveInputStream, file.toPath())
                        // Set executable permissions for the content of the bin folder
                        if (file.parentFile.isDirectory && EXECUTABLE_DIRS.any { file.parentFile.nameWithoutExtension == it }
                        ) {
                            file.setExecutable(true)
                        }
                    }
                }
            }
    }

    private fun getOsName(): String {
        val osName = env.os.lowercase()
        return if (osName.contains("mac os")) "darwin"
        else if (osName.contains("linux")) "linux"
        // TODO - Figure out Windows support?
        else throw IllegalArgumentException("Unsupported OS ${env.os}")
    }

    private fun getArchName(): String {
        return when (env.arch) {
            "aarch64" -> "arm64"
            "amd64", "x86_64" -> "x64"
            else -> throw IllegalArgumentException("Unsupported Architecture ${env.arch}")
        }
    }

    companion object {
        private const val OSS_TOOLS_URL_TEMPLATE =
            "https://github.com/YosysHQ/oss-cad-suite-build/releases/download/{{date}}/oss-cad-suite-{{os}}-{{arch}}-{{minified-date}}.tgz"
        private val EXECUTABLE_DIRS = listOf("bin", "libexec")
    }
}

class UninstallCommand(private val env: Environment) :
    CliktCommand(help = "Uninstall an OSS CAD Suite toolchain") {
    private val version by argument(help = "Toolchain version to uninstall")

    override fun run() {
        val uninstallPath = File("${env.toolchainHome}/$version")
        if (!uninstallPath.isDirectory) {
            println("'$version' is not a valid toolchain directory.")
            exitProcess(1)
        }
        println("Uninstalling '$uninstallPath'")
        uninstallPath.deleteRecursively()
    }
}

class ListCommand(private val env: Environment) : CliktCommand(help = "List installed toolchain versions") {
    override fun run() {
        // Check for default
        val defaultAlias = File("${env.aliasesHome}/default").toPath().toRealPath()

        // Check for installed toolchains
        File(env.toolchainHome)
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

class DefaultCommand(private val env: Environment) :
    CliktCommand(help = "Set an installed toolchain as the default toolchain") {
    private val version by argument(help = "Toolchain version to make default")

    override fun run() {
        val aliasesHome = File(env.aliasesHome)
        aliasesHome.mkdirs()

        val toolchainPath = File("${env.toolchainHome}/$version")
        if (!toolchainPath.isDirectory) {
            println("'$version' is not a valid toolchain directory.")
            exitProcess(1)
        }
        val defaultAlias = File("${env.aliasesHome}/default")
        if (defaultAlias.exists()) {
            defaultAlias.delete()
        }
        println("Setting $version as default.")
        Files.createSymbolicLink(defaultAlias.toPath(), toolchainPath.toPath())
    }
}

class EnvCommand(private val env: Environment) :
    CliktCommand(help = "Print and setup required environment variables for icicle") {
    override fun run() {
        val defaultAlias = File("${env.aliasesHome}/default/").toPath()
        val caches = File(env.cachesHome)
        caches.mkdirs()

        // Clean up old links
        cleanupCaches()

        // Symlink to default
        val shellSessionSymLink = createTempSymbolicLink(caches, defaultAlias)
        val binPath = "${shellSessionSymLink}/oss-cad-suite/bin"
        val libExecPath = "${shellSessionSymLink}/oss-cad-suite/libexec"

        println("export ICICLE_SHELL_PATH=${shellSessionSymLink}")
        println("export ICICLE_HOME=${env.icicleHome}")
        println("export PATH=$binPath:PATH")
        println("export PATH=$libExecPath:PATH")
    }

    private fun cleanupCaches() {
        // TODO
//        val attrs = Files.readAttributes(
//            link,
//            BasicFileAttributes::class.java
//        )
//        val time = attrs.lastAccessTime()
//        println(time)
    }

    private fun createTempSymbolicLink(dir: File, destination: Path): File {
        val path = File.createTempFile("icicle_", "", dir).also(File::delete)
        return Files.createSymbolicLink(path.toPath(), destination).toFile()
    }
}