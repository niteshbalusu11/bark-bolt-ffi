plugins {
    id("com.android.library")
    kotlin("android")
    `maven-publish`
}
android {
    namespace = "io.github.niteshbalusu11.bark"
    compileSdk = 35
    defaultConfig { minSdk = 24; consumerProguardFiles("consumer-rules.pro") }
    sourceSets["main"].apply {
        java.srcDir(rootProject.file("dist/kotlin"))
        jniLibs.srcDir(rootProject.file("dist/android/jniLibs"))
    }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    publishing { singleVariant("release") { withSourcesJar() } }
}
kotlin { jvmToolchain(17) }
dependencies { api("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2") }
afterEvaluate {
    publishing { publications { create<MavenPublication>("release") {
        from(components["release"])
        groupId = project.group.toString(); artifactId = "bark-bolt-ffi-android"; version = project.version.toString()
    } } }
}
// Include handwritten host conveniences alongside generated bindings.
kotlin.sourceSets.getByName("main").kotlin.srcDir(rootProject.file("sdk/kotlin-support"))
