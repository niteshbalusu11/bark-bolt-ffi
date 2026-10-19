# Tasks 04–06: SDK packaging and parity checks

The Rust port plan defines the public interface and runtime invariants. Add
`boltffi.toml`, reproducible scripts under `scripts/`, a root Swift package and
Gradle modules `sdk/android` and `sdk/jvm`. Generated sources live under `dist`;
no hand-edits to generated Swift/Kotlin or JNI.

Build iOS arm64, simulator arm64/x86_64, macOS arm64/x86_64; Android
arm64/armv7/x86_64 API24; JVM desktop natives on each CI host. Mac and Linux JVM
artifacts are distinct classifiers until release assembly merges resource paths.
Pin the CLI exactly to crate 0.31.1 and toolchains. Use release artifacts for
published SDKs; local verification can use debug artifacts first. Set module
BarkBoltFFI and package io.github.niteshbalusu11.bark explicitly.

SwiftPM's root source target uses a local generated XCFramework for local builds;
a release manifest points to a checksummed GitHub asset. Gradle packages generated
Kotlin plus JNI resources into the Android library and desktop JAR. Apps use the
project SDK modules. Tests use the real generated full API and check seed/error,
callback success/failure, full-range values, persistence and notification reuse.
Compare old export names to scanner output with explicit factory mappings;
strict generation must report no omissions. Preserve the old SQLite filename.

Remove .gitlab-ci.yml and obsolete browser docs/source/build commands after the
native checks pass, updating README with only supported consumer products.
