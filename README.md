# bark-bolt-ffi

Swift and Kotlin bindings for the Bark wallet, migrating from UniFFI to BoltFFI.

The project will contain the Rust FFI core, Swift and Kotlin Android/JVM SDKs,
native SwiftUI and Compose example apps, an isolated RegTest environment, and
GitHub Actions for SDK/app builds and tests.

Targets: iOS 16+, macOS 13+, Android API 24+, and JVM 17. Breaking SDK API changes
are allowed during the migration.

## Current status

The native compatibility proof passes in Swift and Kotlin. The production core
still contains the original UniFFI/WASM implementation; the migration and native
apps are not complete. Follow the [production migration checklist](docs/production-migration-todo.md)
for engineering plans, implementation order and acceptance criteria.

- [Migration assessment](docs/boltffi-migration-assessment.md)
- [Runnable native proof](experiments/boltffi/README.md)

## Upstream

Derived from [Bark FFI](https://gitlab.com/ark-bitcoin/bark-ffi); upstream history
and the [MIT license](LICENSE) are preserved. Packaging references come from
[bark-ffi-bindings](https://gitlab.com/ark-bitcoin/bark-ffi-bindings).
RegTest infrastructure will draw on
[Nitro Ark](https://github.com/smolcars/react-native-nitro-ark).
