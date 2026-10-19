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

Device acceptance exposed a hang in Kotlin notification cancellation/reuse.
Minimize this with a real-wallet JVM test (no daemon, persistent temp SQLite;
the offline attempt requires a genesis-block chain request, so use RegTest), then compare host cancellation and explicit
cancellation independently before changing the shared implementation. Preserve a
regression at this actual SDK boundary. Do not bypass this acceptance check.

Confirmed cancellation cause: `if let` held the cancel mutex through tx.send,
which synchronously resumes Kotlin Main.immediate/Unconfined and re-enters
cleanup. An actual JVM RegTest test with Dispatchers.Unconfined times out in
15s before the fix. Take the sender out of the lock before waking or dropping it.

Modern Swift validation: compile the example in Swift6 language mode, not only
with a Swift6 compiler in Swift5 mode. If generated object classes lack Sendable,
add a separate handwritten sdk/swift-support/Concurrency.swift (never edit the
generated file), justified by native Rust Send/Sync objects and atomic handles.
Copy it into each packaged Swift target; test the strict app and actor crossing.
Callback implementations still must synchronize state when called by Rust workers.
