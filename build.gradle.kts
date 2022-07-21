import org.jetbrains.kotlin.gradle.tasks.KotlinCompile
import java.util.Properties
import java.io.FileOutputStream

plugins {
    kotlin("jvm") version "1.7.10"
    kotlin("plugin.serialization") version "1.7.10"
    id("org.graalvm.buildtools.native") version "0.9.13"
}

group = "com.nishtahir.icicle"
version = "0.0.1"

val generatedPropertiesDir = "$buildDir/properties"
sourceSets {
    main {
        resources {
            srcDirs.add(file(generatedPropertiesDir))
            output.dir(generatedPropertiesDir)
        }
    }
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("com.github.ajalt.clikt:clikt:3.5.0")
    implementation("com.github.kittinunf.fuel:fuel:2.3.1")
    implementation("org.apache.commons:commons-compress:1.21")
    testImplementation(kotlin("test"))
}

graalvmNative {
    binaries {
        named("main") {
            imageName.set("icicle")
            mainClass.set("com.nishtahir.icicle.MainKt")
            buildArgs.add("--enable-url-protocols=https")
            javaLauncher.set(javaToolchains.launcherFor {
                languageVersion.set(JavaLanguageVersion.of(11))
                vendor.set(JvmVendorSpec.matching("GraalVM Community"))
            })
            configurationFileDirectories.from(file("./.graal"))
        }
    }
}

tasks.test {
    useJUnitPlatform()
}

tasks.withType<KotlinCompile> {
    kotlinOptions.jvmTarget = "11"
}

tasks.register("generateVersionProperties") {
    doLast {
        val propertiesFile = file("$generatedPropertiesDir/version.properties")
        propertiesFile.parentFile.mkdirs()
        val properties = Properties()
        properties.setProperty("version", "$version")
        val out = FileOutputStream(propertiesFile)
        properties.store(out, null)
    }
}

tasks.named("processResources") {
    dependsOn("generateVersionProperties")
}
