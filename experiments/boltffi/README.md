# BoltFFI native compatibility proof

This throwaway experiment answers whether the difficult parts of Bark's native
API can cross BoltFFI into Swift and Kotlin. It uses an in-memory wallet model,
with no Bark dependency, wallet files, network access, seed, or transaction signing.
The existing SDK and companion bindings repository are unchanged.

## Result

**Passed on 2026-10-18:** both generated consumers compiled and ran successfully
with the unmodified BoltFFI 0.31.1 CLI. No generated bindings were patched.

- Swift: imported the generated SwiftPM package and linked its macOS arm64
  XCFramework slice; executed the checks below.
- Kotlin: compiled with Kotlin 1.9.24 and JVM target 17; executed using the
  packager's desktop JNI library and bundled resource loader, without JNA.
- Android: packaged an arm64-v8a library with the generated JNI bridge and minSdk
  24. This library was built, not executed on Android or published as an AAR.
- Apple: packaged iOS arm64, iOS Simulator arm64 and macOS arm64 static libraries.
  The generated Swift package also compiled for iOS Simulator with an iOS 14
  deployment target. No iOS app or simulator was run.
- Rust: build, formatting and Clippy with warnings denied passed on Rust 1.90.0.

Both consumers assert:

1. Async, fallible factories returning concrete objects, including borrowing an
   onchain object without consuming it.
2. A real Tokio timer firing on the dedicated Rust runtime before returning.
3. Record defaults (`true` and absent optional values), explicit values, UTF-8
   and embedded NUL strings, and the full `u64` range.
4. Retained synchronous callbacks invoked by Rust worker tasks, including
   record/vector arguments, returned strings, and fallible unit results.
5. Both typed callback errors and unexpected foreign exceptions reaching the
   native caller with their messages.
6. A data enum with nested records/vectors crossing in both directions.
7. Notification receive, rejection of concurrent receive, explicit cancellation,
   native task/coroutine cancellation, and reuse after cancellation.
8. Wallet destruction/close and notification stream closure. Swift additionally
   uses a weak reference to assert callback retention and eventual release.

## Run

This configuration selects Apple silicon host/device/simulator slices. Use an
Apple silicon Mac with Xcode, Rust 1.90.0 and its `aarch64-apple-darwin`,
`aarch64-apple-ios`, `aarch64-apple-ios-sim`, and `aarch64-linux-android` targets,
JDK 17, and an installed Android SDK/NDK.

Install or download the [BoltFFI 0.31.1 CLI](https://github.com/boltffi/boltffi/releases/tag/v0.31.1)
and [Kotlin 1.9.24 compiler](https://github.com/JetBrains/kotlin/releases/tag/v1.9.24).
When both are on PATH, run from the repository root:

```sh
./experiments/boltffi/run.sh
```

Or supply explicit executables:

```sh
BOLTFFI=/path/to/boltffi KOTLINC=/path/to/kotlinc ./experiments/boltffi/run.sh
```

The runner fetches the pinned kotlinx-coroutines 1.8.1 JVM jar from Maven Central,
verifies its SHA-256 and uses Cargo.lock for Rust dependencies. Generated sources
and build artifacts remain in ignored directories; compiler installations stay
outside the repository. Expected
runtime output includes `PASS Swift:` and `PASS Kotlin:`. Initial downloads and
cross-compilation require network access.

Observed tooling: Swift 6.4 from Xcode, Kotlin 1.9.24, OpenJDK 17.0.15, Android NDK
28.2.13676358, Rust 1.90.0, BoltFFI 0.31.1. The downloaded macOS arm64 BoltFFI CLI
archive was verified against the release checksum:
`3837df80dae7d8d1d84f6b9c73890b9b6f79b3b74360d9b5a788524740b7bfdf`.

## Decisions validated and remaining work

- Return `Self`/concrete objects instead of exported `Arc<T>`. Internal callbacks
  remain in `Arc<dyn Trait>`.
- Keep `WalletOpenArgs` as a value record and move onchain handles to a borrowed
  factory parameter. Separate factories preserve the optional-onchain use case.
- Use a message-bearing FFI error and convert `UnexpectedFfiCallbackError`.
- Keep the dedicated Tokio runtime/offload pattern. These checks do not establish
  that cancellation aborts a task detached by dropping its Tokio JoinHandle.
- The proof's notification guard clears the cancel sender and the waiting flag
  on **drop**. Production currently clears the sender only after await; foreign
  cancellation skips that code. Carry this cleanup change into the real port.
- Generated SwiftPM output declares **macOS 13**, hardcoded in BoltFFI's packager.
  The companion SDK currently declares macOS 12. Preserving that contract needs
  explicit package handling and testing; this proof does not establish macOS 12
  runtime compatibility.
- Swift 6.4's default package build engine rejected iOS 14 with
  `supported deployment target versions is 15.0 to 27.1.x`. Its native build engine
  compiled the same package for iOS 14; the runner selects that engine for this
  check. That engine is deprecated, so the deployment/build setup still needs
  validation on the release toolchain.

This proves representative FFI shapes, not the entire Bark wallet. It does not
validate real SQLite persistence, chain/network operations, daemon shutdown,
all exported types/methods, mobile runtime behavior, older Swift compilers,
additional ABIs/desktop hosts, release optimization or published AAR/Maven
artifacts. See the [migration assessment](../../docs/boltffi-migration-assessment.md)
for the coordinated two-repository migration scope.
