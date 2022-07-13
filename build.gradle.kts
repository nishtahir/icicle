import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

plugins {
    kotlin("jvm") version "1.7.10"
    kotlin("plugin.serialization") version "1.7.10"
    id("org.graalvm.buildtools.native") version "0.9.4"
}

group = "com.nishtahir"
version = "1.0-SNAPSHOT"

repositories {
    mavenCentral()
}

dependencies {
    implementation("com.charleskorn.kaml:kaml:0.46.0")
    implementation("com.github.ajalt.clikt:clikt:3.5.0")
    implementation("com.github.kittinunf.fuel:fuel:2.3.1")
    implementation("com.jaredrummler:ktsh:1.0.0")
    implementation("org.apache.commons:commons-compress:1.21")
    testImplementation(kotlin("test"))
}

nativeBuild {
    imageName.set("icicle")
    mainClass.set("com.nishtahir.icicle.MainKt")
    buildArgs.add("--enable-url-protocols=https")
    javaLauncher.set(javaToolchains.launcherFor {
        languageVersion.set(JavaLanguageVersion.of(11))
        vendor.set(JvmVendorSpec.matching("GraalVM Community"))
    })
}

tasks.test {
    useJUnitPlatform()
}

tasks.withType<KotlinCompile> {
    kotlinOptions.jvmTarget = "11"
}