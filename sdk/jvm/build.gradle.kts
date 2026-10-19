plugins { kotlin("jvm"); `maven-publish` }
kotlin { jvmToolchain(17) }
val nativeHost = (if (System.getProperty("os.name").startsWith("Mac")) "darwin" else "linux") + "-" +
    (if (System.getProperty("os.arch") in listOf("aarch64", "arm64")) "arm64" else "x86_64")
tasks.jar { archiveBaseName.set("bark-bolt-ffi-jvm-$nativeHost"); archiveVersion.set("0.1.0") }
sourceSets {
    main { kotlin.srcDir(rootProject.file("dist/kotlin")); resources.srcDir(rootProject.file("dist/android/desktopJniLibs")) }
    test { kotlin.srcDir(rootProject.file("tests/kotlin")) }
}
dependencies {
    api("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
    testImplementation(kotlin("test-junit"))
}
tasks.test { inputs.property("regtest", System.getenv("BARK_REGTEST") ?: ""); environment("BARK_REGTEST", System.getenv("BARK_REGTEST") ?: "") }
publishing { publications { create<MavenPublication>("jvm") {
    from(components["java"])
    groupId = "io.github.niteshbalusu11"; artifactId = "bark-bolt-ffi-jvm-$nativeHost"; version = "0.1.0"
} } }

tasks.register<JavaExec>("testPackaged") {
    dependsOn(tasks.testClasses, tasks.jar)
    classpath = files(tasks.jar) + (sourceSets.test.get().runtimeClasspath - sourceSets.main.get().output)
    mainClass.set("org.junit.runner.JUnitCore")
    args("ContractTest")
    environment("BARK_REGTEST", System.getenv("BARK_REGTEST") ?: "")
}
