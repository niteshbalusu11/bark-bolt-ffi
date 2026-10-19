# Task 13: reproducible SwiftPM and release artifacts

After the final Rust code/dependency checks pass, produce optimized full ABI
SDKs. For this private repository, make root SwiftPM consumption self-contained:
copy the generated release XCFramework, FFI headers and Swift source to
sdk/swift and put portable paths in root Package.swift. Each static slice must
remain below GitHub's file size limit. No Git LFS or authenticated release asset
redirect should be required to resolve the package from the Git URL.

Record SHA256 checksums, BoltFFI/Bark/toolchain versions and a hash of Rust
sources plus Cargo.lock/manifest/config in sdk/swift/provenance.json. Add a
verification script so source or generated contract drift fails CI. Run a clean
Swift consumer from the root package, including a separate Git clone, and verify
Android AAR/JVM JAR contain the same crate/API with JNI natives for all promised
ABIs. The app debug test bridge stays disabled in release builds.

Document Swift/Kotlin breaking factory/error/default/ownership changes, precise
CI/device evidence, commands and any uncovered external service protocols.
Do not publish to App Store, Play Store or Maven Central as part of validation.
