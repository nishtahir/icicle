package com.nishtahir.icicle

import kotlinx.serialization.Serializable

@Serializable
data class Manifest(
    val toolchain: String,
    val toolchainHome: String = "${System.getProperty("user.home")}/.icicle",
    val shell: String = "/bin/bash",
    val scripts: Map<String, String>,
    val os: String = System.getProperty("os.name"),
    val arch: String = System.getProperty("os.arch"),
)