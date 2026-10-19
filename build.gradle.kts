plugins {
    id("com.android.library") version "8.9.2" apply false
    id("com.android.application") version "8.9.2" apply false
    kotlin("android") version "2.1.20" apply false
    kotlin("jvm") version "2.1.20" apply false
    id("org.jetbrains.kotlin.plugin.compose") version "2.1.20" apply false
}

val cargoPackage = rootProject.file("Cargo.toml").readText().substringAfter("[package]").substringBefore("\n[")
val sdkVersion = Regex("""(?m)^version\s*=\s*"([^"]+)"\s*$""").find(cargoPackage)?.groupValues?.get(1)
    ?: error("Cargo.toml must declare the SDK package version")
allprojects { group = "io.github.niteshbalusu11"; version = sdkVersion }
