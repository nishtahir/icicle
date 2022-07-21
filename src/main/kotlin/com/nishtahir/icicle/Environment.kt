package com.nishtahir.icicle

import java.io.File
import java.util.*
import kotlin.io.path.Path

data class Environment(
    val icicleHome: String,
    val shellPath: String?,
    val os: String,
    val arch: String,
    val version: String,
) {
    val toolchainHome = "$icicleHome/toolchains"
    val aliasesHome = "$icicleHome/aliases"
    val cachesHome = "$icicleHome/caches"

    fun toolchainFileVersion(): String? {
        return toolChainFile()?.readText()?.trim()
    }

    private fun toolChainFile(): File? {
        val cwd = System.getProperty("user.dir")
        val toolchainFile = Path(cwd, TOOLCHAIN_FILE_NAME).toFile()
        return when {
            !toolchainFile.exists() -> null
            else -> toolchainFile
        }
    }

    companion object {
        const val TOOLCHAIN_FILE_NAME = ".icicle-toolchain"

        fun create(): Environment {
            val shellPath = System.getenv("ICICLE_SHELL_PATH")?.trim()
            val icicleHome = System.getenv("ICICLE_HOME")
                ?.trim()
                ?.removeTrailingSlash()
                ?: "${System.getProperty("user.home")}/.icicle"

            return Environment(
                icicleHome = icicleHome,
                shellPath = shellPath,
                os = System.getProperty("os.name"),
                arch = System.getProperty("os.arch"),
                version = applicationVersion()
            )
        }

        private fun applicationVersion(): String {
            val properties = Properties()
            properties.load(Environment::class.java.getResourceAsStream("/version.properties"))
            return properties.getProperty("version")
        }

        private fun String.removeTrailingSlash(): String {
            if (this.endsWith("/")) {
                return this.substring(0, this.length - 1)
            }
            return this
        }
    }
}
