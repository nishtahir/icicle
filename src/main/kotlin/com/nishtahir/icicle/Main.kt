package com.nishtahir.icicle

import com.charleskorn.kaml.Yaml
import com.github.ajalt.clikt.core.subcommands
import java.io.File
import java.util.regex.Pattern

fun main(argv: Array<out String>) {
    val cwd = System.getProperty("user.dir")
    val manifestFile = findManifestFile(cwd) ?: throw IllegalStateException("Failed to find 'icicle.yml'.")

    val yaml = createYaml()
    val manifest = yaml.decodeFromStream(Manifest.serializer(), manifestFile.inputStream())

    // Setup CLI
    val downloader = ToolchainDownloader(manifest)
    Cli().subcommands(InstallCommand(manifest, downloader))
        .subcommands(UninstallCommand(manifest))
        .subcommands(ListCommand(manifest))
        .subcommands(DefaultCommand(manifest))
        .subcommands(RunCommand(manifest))
        .main(argv)
}

val MANIFEST_FILE_PATTERN: Pattern = Pattern.compile("icicle\\.ya?ml", Pattern.CASE_INSENSITIVE)

private fun findManifestFile(cwd: String): File? {
    return File(cwd).listFiles()?.find {
        MANIFEST_FILE_PATTERN.matcher(it.name).matches()
    }
}

private fun createYaml(): Yaml {
    return Yaml(configuration = Yaml.default.configuration.copy(strictMode = false))
}
