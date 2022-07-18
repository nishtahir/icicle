package com.nishtahir.icicle

import kotlinx.serialization.Serializable

@Serializable
data class Manifest(
    val toolchain: String? = null,
    val icicleHome: String = "${System.getProperty("user.home")}/.icicle",
    val shell: String = "/bin/bash",
    val scripts: Map<String, String> = emptyMap(),
    val os: String = System.getProperty("os.name"),
    val arch: String = System.getProperty("os.arch"),
) {
    val toolchainHome = "$icicleHome/toolchains"
    val aliasesHome = "$icicleHome/aliases"
    val toolchainPath = "$toolchainHome/$toolchain"

    companion object {
        fun default(): Manifest {
            return Manifest()
        }
    }
}
