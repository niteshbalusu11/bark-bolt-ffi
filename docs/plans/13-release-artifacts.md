# Release artifacts and clean consumers

Paths: root Package.swift, sdk/swift, scripts/swift-artifact.py,
scripts/test-swift-release.py, SDK Gradle publications, artifact checks, CI,
README and migration/validation documentation.

Produce optimized full-matrix SDKs after Rust/API validation. The repository is
public at smolcars/bark-bolt-ffi. Root SwiftPM uses a versioned public GitHub
release XCFramework ZIP and SHA256; commit only generated Swift/C source and
provenance. Static archives exceed GitHub's normal Git file limit. Local app
builds continue using dist/apple independently of release availability.

Before staging, verify all five Apple platform/architecture pairs against their
Cargo release archives. Check Mach-O minimum deployment commands for every
native object: iOS16 and macOS13. Set MACOSX_DEPLOYMENT_TARGET=13.0 on Darwin,
including the JVM JNI build, so native C dependencies inherit the minimum.
Record source/config/build-script, generated files, manifest and archive hashes.
CI checks committed provenance and newly built Apple deployment targets.

The deployment gate first reproduced zstd objects targeting macOS27, then the
explicit macOS13 build setting corrected the archive. Require a fresh ZIP
consumer to link without newer-macOS warnings and pass existing Swift contracts.
After publication, run test-swift-release.py against the full pushed Git SHA;
this exercises real Git resolution, public download/checksum, linkage, callback
conversion and handle release. This is a manual release gate. Normal push/PR CI
uses fresh local SDKs and must support commits before a release exists.

For Kotlin, inspect AAR public classes and ELF machine IDs for the three Android
ABIs. Run existing contracts through the actual JVM JAR, removing main output
directories from its runtime classpath. Publish both modules to MavenLocal and
run independent JVM/Android consumers, including a minified APK with exactly the
three supported ABIs. Collect Linux JVM output from the final successful hosted
run and the local macOS JVM artifact. Upload versioned AAR/JAR assets, provenance
and checksums alongside the Swift archive.

Publish v0.1.0 at the validated implementation SHA only after hosted SDK/app and
full cross-platform RegTest checks pass. Then run the clean Git consumer before
final completion. Preserve release asset checksums after publication. Store
publishing and Maven Central publication are outside this migration.

Finish the checklist with actual commands, source/CI/release links and remaining
runtime limits. Remove superseded progress prose, keep engineering plans and
durable evidence, and check relative links/whitespace. Required build/RegTest
helpers and generated SDK contracts remain. No wallet logic changes in this task.

Public-consumer isolation: the first public SwiftPM consumer consults two local
GitHub Keychain entries and stalls at artifact download, while an anonymous HTTP
request returns200 with the correct content length. Disable Keychain/netrc lookup
only for the clean release-check process, without changing local credentials.
Rerun the actual Git/download/link/contracts check and record its result. This
changes the validation harness, not SDK source or published binary checksums.

Outcome: all hosted SDK/app jobs and paired native RegTest passed. Release0.1.0
is published with checksum-verified assets. The anonymous Git-URL consumer
downloads the real XCFramework, compiles/links and passes all three contracts.
Record detailed evidence in validation/2026-10-19-master.md and close all tasks.
