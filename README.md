# Bark BoltFFI

Native **Swift and Kotlin** bindings for [Bark master](https://gitlab.com/ark-bitcoin/bark/-/commit/bfbe6b7c3088a09dd4e81c4ac24cdc763a627a8c),
using BoltFFI 0.31.1. Includes SwiftUI and Jetpack Compose example wallets,
isolated Bitcoin/Ark/Lightning RegTest infrastructure and GitHub Actions.

See [installation and breaking changes](docs/migration.md), the
[production checklist](docs/production-migration-todo.md) and
[validation evidence](docs/validation/2026-10-19-master.md). The examples perform wallet
operations through the generated SDKs; RegTest helpers supply test coins and blocks.

Targets: iOS 16+, macOS 13+, Android API24+, JVM17. Swift and Kotlin APIs may
break from the original UniFFI API. Rust retains upstream wallet logic,
transaction/PSBT validation, SQLite persistence and native datadir locking.

## Install

Swift: add `https://github.com/smolcars/bark-bolt-ffi` in Xcode and link
`BarkBoltFFI`. The root SwiftPM package uses the versioned, checksummed release
XCFramework. Kotlin: build the AAR/JVM JAR below, or download Actions artifacts;
[the migration guide](docs/migration.md) documents local Maven publication.

## Local build

Install Rust 1.90, protobuf (`protoc`), JDK17 and the Android SDK/NDK28.2.13676358.
Apple builds need Xcode; the example project uses XcodeGen. Homebrew tools must
be on PATH. Install the pinned BoltFFI CLI with:

```sh
scripts/install-boltffi.sh
export BOLTFFI="$PWD/target/tools/boltffi"
scripts/build-sdk.sh apple --overlay boltffi.local.toml
scripts/build-sdk.sh android --overlay boltffi.local.toml
```

The local overlay selects Apple silicon/Android arm64. Omit it for the full ABI
matrix and add `--release` for optimized SDKs. Generated bindings are never
patched. SwiftPM package: `dist/apple`; Android AAR: `sdk/android/build/outputs/aar`;
JVM JAR: `sdk/jvm/build/libs`. Compile Kotlin artifacts with:

```sh
./gradlew :sdk:android:assembleRelease :sdk:jvm:test :examples:android:assembleDebug
swift test --package-path tests/swift
xcodegen generate --spec examples/ios/project.yml
```

Open `examples/ios/BarkExample.xcodeproj` and build the BarkExample scheme for an
iPhone simulator. Simulator Keychain storage uses local ad hoc signing without
an Apple developer account. Device signing needs your normal Apple team setup.

## RegTest

```sh
python3 scripts/regtest.py start
python3 scripts/regtest.py reference-smoke
python3 scripts/regtest.py status
python3 scripts/regtest.py mine 6
python3 scripts/regtest.py fund ADDRESS 0.1
python3 scripts/regtest.py stop
```

Docker Compose uses isolated `bark-bolt-ffi` volumes. Commands never reset wallets.
Bitcoin RPC is `http://127.0.0.1:19443` (`second` / `ark`, RegTest credentials only),
Ark is `http://127.0.0.1:3635`. Android reaches these through `adb reverse`; iOS
simulators use host loopback. Fund an app's onchain address, mine, sync and board.
The SDK performs wallet operations; the helper only funds/mines test coins.

With both apps installed, run real cross-platform acceptance:

```sh
python3 scripts/app-regtest.py --ios-device SIMULATOR_UDID --android-device emulator-5554
python3 scripts/app-regtest.py --extended --ios-device SIMULATOR_UDID
```

Each run uses a new private app wallet account. Recovery phrases stay in Keychain
or Android Keystore-encrypted private storage and never enter test output. The
debug command bridge is excluded/disabled in release apps.

## CI

[SDK and native apps](.github/workflows/ci.yml) builds Rust, SwiftPM/XCFramework,
Android AAR/JVM JAR and both apps on GitHub-hosted Linux/macOS runners. It executes
SDK contracts, iOS UI tests and Android UI/RegTest tests.

[Full cross-platform acceptance](.github/workflows/integration.yml) is manually
dispatched on a macOS ARM64 runner labelled `bark-bolt-regtest` with Xcode,
Docker, JDK17, Android SDK/NDK, a booted simulator/emulator, `IOS_DEVICE` and
`ANDROID_SERIAL`. This explicitly avoids assuming Linux containers work on a
hosted macOS runner. Tests use a separate Compose project/ports. The validation
record links actual completed runs. Full integration passed in
[Actions39195200829](https://github.com/smolcars/bark-bolt-ffi/actions/runs/39195200829);
see [device/runner setup](docs/devices.md) to repeat it.

## Upstream

Forked from [ark-bitcoin/bark-ffi](https://gitlab.com/ark-bitcoin/bark-ffi).
Original history and [MIT license](LICENSE) are retained. Nitro Ark's RegTest
layout informed the isolated backend; its reference checkout/volumes are untouched.
