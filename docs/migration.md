# Installing and migrating

This SDK exposes Bark master `bfbe6b7c3088a09dd4e81c4ac24cdc763a627a8c`
(0.7.1-dev,2026-10-16) through BoltFFI0.31.1. Swift and Kotlin are the
supported consumer languages. The Rust core remains in this repository.

## Swift

Add `https://github.com/smolcars/bark-bolt-ffi` in Xcode's package dependencies,
select version0.1.0, and link the `BarkBoltFFI` product. In a package manifest:

```swift
.package(url: "https://github.com/smolcars/bark-bolt-ffi.git", from: "0.1.0")
// Target dependency:
.product(name: "BarkBoltFFI", package: "bark-bolt-ffi")
```

Import `BarkBoltFFI`. The root manifest downloads the release XCFramework and
checks its SHA256. Generated Swift/C sources and build provenance are committed
under `sdk/swift`. No Rust build is needed by a released Swift consumer.
Local examples instead consume `dist/apple` produced by `scripts/build-sdk.sh`.

## Kotlin

Build Android AAR and the current host's JVM JAR with:

```sh
scripts/install-boltffi.sh
BOLTFFI="$PWD/target/tools/boltffi" scripts/build-sdk.sh android --release
./gradlew :sdk:android:assembleRelease :sdk:jvm:jar :sdk:jvm:testPackaged
./gradlew :sdk:android:publishToMavenLocal :sdk:jvm:publishToMavenLocal
```

Use `mavenLocal()` with `io.github.niteshbalusu11:bark-bolt-ffi-android:0.1.0`,
or the host-specific `bark-bolt-ffi-jvm-darwin-arm64` / `bark-bolt-ffi-jvm-linux-x86_64`
artifact. Import `io.github.niteshbalusu11.bark.*`. The Maven POM supplies the
coroutines dependency. The Android AAR embeds all three JNI ABIs and consumer
R8 rules. JVM JARs contain the native library for the build host; choose a JAR
matching the deployment OS/architecture. Linux CI builds on Ubuntu24.04; rebuild
locally when targeting an older libc. Maven Central publication is separate.
GitHub Actions also uploads the built AAR/JAR and example APKs.

## Breaking API and ownership changes

| Area | New contract |
| --- | --- |
| Swift module | `BarkBoltFFI` |
| Kotlin namespace | `io.github.niteshbalusu11.bark` |
| Errors | `BarkError.message` contains the complete Rust error chain; update catches that matched the old UniFFI enum. Unexpected callback exceptions are converted to this error. |
| Factories | BoltFFI uses concrete object factories. Swift `Wallet.openWithOnchain(...)`, Kotlin `Wallet.openWithOnchain(...)` construct a wallet with an existing onchain handle. |
| Open arguments | `WalletOpenArgs` contains values only. Pass the onchain object separately to `openWithOnchain`; `open` supports wallets without one. |
| Defaults | Optional config fields use nil/null defaults. Check generated declarations for unsigned integer and enum spellings. |
| Kotlin resources | Object wrappers implement `close()`. Keep them alive while needed and close notification/onchain/wallet wrappers during teardown. In-flight native work retains its own references. Calling a closed wrapper fails. |
| Swift resources | ARC releases native handles. The separate concurrency adapter marks synchronized native object wrappers Sendable for Swift6 actor crossing. |
| Callbacks | Implement all methods of `CustomOnchainWalletCallbacks` or `BarkLogger`. Native workers can call them off the UI thread; synchronize mutable state. Native ownership retains callbacks until their owner/logger registration is released. |
| Notifications | One pending receive per holder. Explicit cancellation or host-task cancellation releases that wait; reuse is supported. Concurrent waits return an error. |
| Cancellation | Cancelling a foreign await does not roll back a submitted wallet operation. Tokio work already submitted may finish; sync/reconcile the wallet before retrying a payment. |
| Daemon | Stop with `stopDaemonWait()` before replacing a wallet. Dropping the last native wallet also stops its daemon. |

All128 original callable exports,40 records,14 enums and both callback traits
are tracked in `api-baseline.json` and checked against strict host generation.
Generated files are never patched; the Swift concurrency adapter is handwritten
in `sdk/swift-support`. There is no UniFFI runtime, JNA or browser product.

SQLite uses the existing `bark.sqlite` filename. Open an existing wallet with
its original seed/network/datadir; do not silently create a replacement seed.
The apps keep seeds in device-only Keychain or Keystore-encrypted private
storage. A recovered seed can recover funds into a new database while historic
movement metadata may be absent. The example recovery path scans onchain from
height0. See each `WalletModel` for the actual open/shutdown/recovery sequence.

## Platform and verification scope

| Product | Minimum | Packaged architectures | Runtime evidence |
| --- | --- | --- | --- |
| iOS | 16 | device arm64; simulator arm64/x86_64 | iOS27 ARM64 simulator |
| macOS | 13 | arm64/x86_64 | macOS ARM64 Swift contracts |
| Android | API24 | arm64-v8a/armeabi-v7a/x86_64 | API35 ARM64 locally; x86_64 CI gate |
| Kotlin/JVM | Java17 | build-host native JAR | macOS ARM64 contracts; Linux CI gate |

Minimum OS versions and the other packaged ABIs are compile targets; this does
not claim physical-device or every-OS runtime testing. The example apps are
RegTest tools with debug-only local HTTP networking and test command bridges.
They are not store-distributed wallet products.

Live acceptance covers boarding, two-way Ark transfers, BOLT11 send/receive,
notifications, refresh, offboarding, unilateral exits/claims, restart and funded
seed recovery. LNURL/LNAddress and BOLT12 exports remain available and retain
upstream validation, but no external provider acceptance fixture was run.
See `validation/2026-10-19-master.md` for current-master results and CI links;
`validation/2026-10-18.md` retains prior release0.7.1 evidence.

## Current-master changes

Upstream master adds `CustomOnchainWalletCallbacks.signPsbt`: sign/finalize the
PSBT without storing it. `finishPsbt` still signs and stores; implement both.
Unsigned transaction inputs/outputs are validated across either callback.
`ExitState`/`ExitStateKind` add VtxoSwept, preserving its spent outpoints. The
emergency fee estimate removes the obsolete `fundable` field and accepts an
optional finite non-negative fee margin; upstream defaults to1.2. Heights remain
unsigned32-bit at the FFI; relative block deltas above65535 return a typed error.

RegTest follows Nitro master's Bitcoin/Postgres/CLN/LND/server/CLI topology, with
this project's isolated storage and ports. It uses the user's multi-architecture
`niteshbalusu/bark:nightly-2026-10-19` (same master commit) and latest available
`niteshbalusu/captaind:nightly-2026-10-18` (0.7.1 release), each pinned by digest.
The server requires Bitcoin31, which the stack pins. The server image pin was
explicitly confirmed by the user. The dates are registry UTC
build dates; the Bark image was published on October18 in America/New_York.
