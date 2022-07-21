package com.nishtahir.icicle

import java.io.File
import kotlin.io.path.Path

data class Environment(
    val icicleHome: String,
    val os: String,
    val arch: String,
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
            val icicleHome = System.getenv("ICICLE_HOME")
                ?.trim()
                ?.removeTrailingSlash()
                ?: "${System.getProperty("user.home")}/.icicle"
            return Environment(
                icicleHome = icicleHome,
                os = System.getProperty("os.name"),
                arch = System.getProperty("os.arch")
            )
        }

        fun String.removeTrailingSlash(): String {
            if (this.endsWith("/")) {
                return this.substring(0, this.length - 1);
            }
            return this
        }
    }
}