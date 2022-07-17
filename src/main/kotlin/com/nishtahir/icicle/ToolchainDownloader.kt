package com.nishtahir.icicle

import com.github.kittinunf.fuel.Fuel
import org.apache.commons.compress.archivers.tar.TarArchiveInputStream
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream
import java.io.BufferedInputStream
import java.io.File
import java.lang.IllegalArgumentException
import java.net.URI
import java.nio.file.Files
import kotlin.math.roundToInt

class ToolchainDownloader(private val manifest: Manifest) {

    /**
     * Versions are provided as dates
     */
    fun downloadAndExtractToolchain(version: String) {
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
        val toolchainHome = File(manifest.toolchainHome)
        if (!toolchainHome.exists()) {
            toolchainHome.mkdirs()
        }

        val toolchainDir = File("${manifest.toolchainHome}/$toolchain")
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
        val osName = manifest.os.lowercase()
        return if (osName.contains("mac os")) "darwin"
        else if (osName.contains("linux")) "linux"
        // TODO - Add Windows support
        else throw IllegalArgumentException("Unsupported OS ${manifest.os}")
    }

    private fun getArchName(): String {
        return when (manifest.arch) {
            "aarch64" -> "arm64"
            "amd64", "x86_64" -> "x64"
            else -> throw IllegalArgumentException("Unsupported Architecture ${manifest.arch}")
        }
    }

    companion object {
        private const val OSS_TOOLS_URL_TEMPLATE =
            "https://github.com/YosysHQ/oss-cad-suite-build/releases/download/{{date}}/oss-cad-suite-{{os}}-{{arch}}-{{minified-date}}.tgz"
        private val EXECUTABLE_DIRS = listOf("bin", "libexec")
    }
}
