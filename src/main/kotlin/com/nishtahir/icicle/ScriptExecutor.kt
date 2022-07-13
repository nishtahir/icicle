package com.nishtahir.icicle

import com.jaredrummler.ktsh.Shell
import kotlin.system.exitProcess

class ScriptExecutor(private val manifest: Manifest) {
    private val environment = "${manifest.toolchainHome}/${manifest.toolchain}/oss-cad-suite/environment"
    private val shell = Shell(manifest.shell)
    fun executeScript(script: String) {
        try {
            // Source the environment so that we have env vars set correctly
            var result = shell.run("source $environment")
            if (!result.isSuccess) {
                println(result.stderr())
                exitProcess(1)
            }

            result = shell.run(script)
            if (result.isSuccess) {
                println(result.stdout())
            } else {
                println(result.stderr())
                exitProcess(1)
            }
        } finally {
            shell.shutdown()
        }
    }
}
