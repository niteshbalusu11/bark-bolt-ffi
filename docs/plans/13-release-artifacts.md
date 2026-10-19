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

The first optimized XCFramework is 550MB because Rust static archives retain
symbol/debug sections. Verify every original thin slice against Cargo release
output before copying; remove only debug symbols from the staged copies using
`xcrun strip -S`, preserving public symbols and leaving Cargo output untouched.
Check each resulting universal archive stays below GitHub's 100MiB file limit,
record the stripped artifact hashes, and link/run a fresh Swift consumer to
validate the packaged symbols. This is packaging only, with no Rust/API change.

Repository state changed externally during execution: GitHub now resolves it as
public `smolcars/bark-bolt-ffi`. Use that canonical URL in origin/docs/runner.
The universal archives still exceed100MiB after debug stripping. Since release
assets are now publicly downloadable, use the standard SwiftPM URL/checksum
binary target and publish the full optimized XCFramework ZIP as a GitHub release
asset. Commit generated Swift/C sources and source/artifact provenance, not large
static archives. Verify checksum/metadata in CI, and resolve/run a clean consumer
from the Git URL after publishing v0.1.0. Keep local dist/apple build workflows
independent of release download availability.

Consumer validation will run the existing Swift contract suite from a clean
external temporary package with a Git revision dependency on this repository.
This exercises public release download/checksum, module linkage, generated
error/callback conversions and release callbacks, rather than merely parsing the
manifest. Add its invocation after Apple local SDK tests in hosted CI once the
release asset is published. Rust CI verifies committed source/provenance hashes.
Keep Maven artifacts as workflow outputs and support local Maven publication;
external Maven Central credentials/publication are outside this migration.

Kotlin package checks will inspect AAR entries and ELF architecture for all three
Android ABIs plus the compiled public API. Run existing JVM contracts through
the built JAR (remove main class directories from the runtime classpath) to prove
that the packaged desktop loader/natives work. Include these in hosted checks,
and compile the minified Android release example to validate consumer keep rules.

Keep the clean Git-URL consumer as an actual release check after asset publication,
not a push/PR CI step: normal CI builds and tests fresh local SDK artifacts, and
must not depend on a public binary release existing for an unreleased commit.
Rust CI verifies committed source/provenance; hosted Apple runs the same contract
suite against the newly generated SDK. Run test-swift-release.py manually against
the pushed release revision and record its result before final completion.

Deployment gate finding: the clean ZIP consumer's linker warns that zstd C
objects were built for macOS27 while the package advertises13. Set
MACOSX_DEPLOYMENT_TARGET=13.0 for Darwin SDK builds (including JVM desktop JNI),
so native C dependencies inherit the declared minimum. Record build-sdk.sh in
the source provenance hash and gate every Apple archive's Mach-O deployment load
commands against iOS16/macOS13 before staging. First run the gate on the existing
archive to capture failure, then rebuild/restage and require no newer-OS linker
warnings in a fresh consumer. This is a binary compatibility fix, not a wallet
logic change. Re-run packaged contracts after rebuilding affected natives.
