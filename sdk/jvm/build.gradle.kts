plugins { kotlin("jvm"); `maven-publish` }
kotlin { jvmToolchain(17) }
sourceSets {
    main { kotlin.srcDir(rootProject.file("dist/kotlin")); resources.srcDir(rootProject.file("dist/android/desktopJniLibs")) }
    test { kotlin.srcDir(rootProject.file("tests/kotlin")) }
}
dependencies {
    api("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
    testImplementation(kotlin("test-junit"))
}
tasks.test { environment("BARK_REGTEST", System.getenv("BARK_REGTEST") ?: "") }
publishing { publications { create<MavenPublication>("jvm") {
    from(components["java"])
    groupId = "io.github.niteshbalusu11"; artifactId = "bark-bolt-ffi-jvm"; version = "0.1.0"
} } }
