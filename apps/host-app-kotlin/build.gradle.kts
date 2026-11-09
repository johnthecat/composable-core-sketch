// The Kotlin host app on the JVM. `make kotlin` builds the host library from
// `apps/host-app-native`, generates its UniFFI bindings into
// `build/generated/uniffi`, and runs the app, which loads the library through
// JNA.

import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    kotlin("jvm") version "2.4.10"
    application
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("net.java.dev.jna:jna:5.17.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
}

java {
    targetCompatibility = JavaVersion.VERSION_21
    sourceCompatibility = JavaVersion.VERSION_21
}

kotlin {
    compilerOptions.jvmTarget = JvmTarget.JVM_21
    sourceSets.main {
        kotlin.srcDir("build/generated/uniffi")
    }
}

application {
    mainClass = "MainKt"
}

tasks.named<JavaExec>("run") {
    // JNA loads the library through `System.load`, a restricted method.
    jvmArgs("--enable-native-access=ALL-UNNAMED")
    // The library `cargo build -p host-app-native` writes.
    systemProperty("jna.library.path", rootDir.resolve("../../target/debug").canonicalPath)
}
