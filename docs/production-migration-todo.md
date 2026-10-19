# Production migration checklist

Status: the full native API is ported to BoltFFI and both apps build and open
real RegTest wallets. Packaging, end-to-end transactions and actual Actions runs
are in progress; unchecked acceptance gates below are still outstanding.

## Agreed decisions

- New private GitHub repository: https://github.com/niteshbalusu11/bark-bolt-ffi,
  created in the authenticated GitHub account; local checkout: bark-bolt-ffi.
- One repository for the Rust FFI core, Swift SDK, Kotlin Android/JVM SDKs,
  native example apps, RegTest infrastructure and GitHub Actions.
- Consumer languages: Swift and Kotlin only. Use SwiftUI and Jetpack Compose.
  Breaking API changes are allowed; no Nitro/React Native schema compatibility.
- Targets: iOS 16+, macOS 13+, Android API 24+, JVM 17. Retain the existing
  native ABI matrix unless a documented platform decision changes it.
- Start with Bark 0.7.1 and BoltFFI 0.31.1 and reproducible dependency pins.
  Confirm server compatibility before considering a separate Bark upgrade.
- Preserve the complete API: callbacks, logging, notifications, Lightning,
  recovery, boarding, refresh, offboarding, exits and onchain operations.
- Actually run both apps on simulator/emulator against RegTest, and create
  GitHub Actions that build SDKs/apps and execute automated checks.

## Rule for every task

Before implementation, refine its engineering plan with exact affected paths,
API/ownership/concurrency decisions, invariants and validation commands. Implement,
run acceptance checks and record evidence before checking it off. If the scope
changes, revise the plan before the next code change. Routine tasks do not need
another approval pause.

Each task's completion record must contain:

```text
Engineering plan and affected paths:
Implemented:
Validation commands and results:
Artifacts / device or CI run evidence:
Remaining limits:
```

Compilation, simulator execution, RegTest success and hosted CI success are
different claims. Record them separately; do not mark one from evidence of another.

## Reference inputs and known setup gaps

- Core source: ark-bitcoin/bark-ffi at
  d3c0f38f29cca1e79f6c81492725acece1f5b8ef. Upstream history and MIT license
  are retained in this checkout.
- Companion packaging reference: ark-bitcoin/bark-ffi-bindings at
  157b0fcda0a0a5782ad2904ed3002a7080293047. Import only needed Swift/Kotlin
  code and shared tooling.
- Nitro Ark exists at
  /Users/niteshchowdharybalusu/Documents/react-native-nitro-ark, local commit
  19b047e16b09bbeabac6ac1bd284e6bb14f4f76f. Inspected remote master:
  988b4f54fb642a42951e685247085a9aff90c4fc.
- Its scripts/docker-compose.yml, scripts/ark-dev.sh and justfile describe
  Bitcoin Core, Postgres, CLN, LND, an Ark server and a Bark CLI. Reuse the
  infrastructure approach, not its React Native binding architecture.
- Nitro uses September nightly server/CLI images and a different Bark revision;
  Compose references a missing captaind.toml. Its bootstrap resets a fixed
  scripts_bark volume. Supply compatible configuration, pin versions, and use
  isolated project names/ports/volumes instead of that reset path.
- Docker/OrbStack is available. An iOS simulator is available. Android SDK/NDK
  exists, but its emulator package and a system image still need installation.
- The [native compatibility proof](../experiments/boltffi/README.md) is complete;
  real Bark wallet, mobile app and RegTest behavior remain to be validated.

## Tasks

### 00 — Bootstrap the new project

- [x] Publish the initial source and plan commit.

**Engineering plan:** create the requested repository; preserve upstream history,
license and compatibility evidence; establish main, GitHub origin and GitLab
upstream; document the migration's current incomplete state.

**Implement:** add README, this checklist and AGENTS.md; commit/push the baseline
and verify the remote commit. Keep original source/Nitro checkouts unchanged.

**Done when:** the private GitHub repo contains the baseline, license and plan,
and local/remote commit IDs agree.

### 01 — Establish the baseline and complete API inventory

- [x] Reproduce baseline native checks and map every public API.

**Engineering plan:** inspect src/, Cargo.toml and companion SDK contracts.
Confirm the initial inventory: 128 callable exports, 40 records, 14 data enums,
3 objects and 2 callback traits. Map defaults, errors, ownership and old/new names.

**Implement:** capture Rust build/test/Clippy outcomes, pin toolchains and record
an API parity/test checklist grouped by wallet feature.

**Done when:** every old export has a planned Swift/Kotlin counterpart and
validation strategy; baseline failures are separated from migration regressions.

### 02 — Build a reproducible RegTest backend

- [ ] Boot compatible, isolated Bitcoin/Ark/Lightning services.

**Engineering plan:** adapt Nitro's setup into regtest/ and lifecycle scripts.
Choose server/CLI versions compatible with Bark 0.7.1, supply missing config and
pin images or source revisions. Define readiness probes, funding, mining,
Lightning liquidity, endpoint configuration and cleanup before coding.

**Implement:** provide bounded start/wait/status/fund/mine/logs/stop commands;
bootstrap mature coins, server funding and CLN/LND channels. Use separate project
names/ports/test volumes. Parameterize iOS simulator, Android emulator and CI
endpoints; keep local HTTP/RPC settings in RegTest/debug configuration.

**Done when:** a fresh stack boots, a reference Bark wallet performs an actual
Ark operation, Lightning channels are funded, repeated setup works, and teardown
does not touch existing Nitro or unrelated wallet/database volumes.

### 03 — Port the full Rust binding layer

- [ ] Replace production UniFFI exports with BoltFFI across the complete API.

**Engineering plan:** retain the core/native adapter seam and Bark revision.
Specify concrete object returns, separate onchain factory arguments, message
errors, callback error conversion, data/default attributes and required Copy
derives before changing src/ and the manifest.

**Implement:** migrate all DTOs/enums, free functions, Wallet/OnchainWallet APIs,
all ten custom-wallet callbacks, logging and notifications. Preserve SQLite,
TLS/BDK, datadir/locking and transaction/PSBT/network validation. Pin crate/CLI
versions and Cargo.lock.

**Done when:** the real crate builds and relevant Rust tests/Clippy pass.
Strict Swift/Kotlin generation has no skipped declarations; every inventory
entry has an implemented counterpart.

### 04 — Verify persistence and async/object lifecycle

- [ ] Test the real SDK's persistence, callbacks, cancellation and daemon shutdown.

**Engineering plan:** define callback/handle retention and release, in-flight
object lifetime, Tokio behavior, notification reuse and shutdown. Foreign task
cancellation must not imply rollback of an already-started payment.

**Implement:** move cancel-slot cleanup into a Drop-safe path; test reopen/restore,
SQLite filename compatibility, daemon stop-and-wait, callback errors/cause
messages, logger replacement/removal, concurrent receives, explicit cancellation,
foreign cancellation and reuse. Remove the React Native logger leak workaround
once native lifetime tests establish safe release.

**Done when:** Rust and both language tests cover these paths on the real SDK,
including in-flight release and failures; no persistence is silently reset.

### 05 — Package usable Swift and Kotlin SDKs

- [ ] Produce SwiftPM/XCFramework, Android AAR and Kotlin/JVM artifacts.

**Engineering plan:** settle layout, package names, deployment targets, ABI matrix,
JNI loading and debug/release artifact flow. Root SwiftPM metadata must work from
the Git URL, and apps must consume newly built local artifacts during development.

**Implement:** build iOS device/simulator and macOS slices, SwiftPM, Android JNI/AAR,
and JVM JAR with desktop natives. Replace JNA with JNI. Add contract tests for
data/enums/defaults/errors and API parity in both languages.

**Done when:** clean consumers compile and run from actual packaged artifacts;
release libraries load, full-range u64/nested values round-trip, and examples
cannot silently resolve an old UniFFI binary.

### 06 — Remove other languages and old infrastructure

- [ ] Leave a Swift/Kotlin-only product with no active UniFFI or browser layer.

**Engineering plan:** identify direct features/consumers to remove; retain Rust
dependencies still needed for metadata or wallet behavior. Import only required
Swift/Kotlin companion tooling and discard its Go-dependent release graph.

**Implement:** remove WASM code, UniFFI scaffolding/features/version-tag conventions,
old GitLab CI and unused build targets. Omit Dart, Go, React Native, browser, npm
and pub tooling. Update supported targets and build commands.

**Done when:** only Swift/Kotlin consumer products remain and native checks pass.
Transitive Cargo dependency names alone do not imply an unsupported product.

### 07 — Build the native iOS example

- [ ] Create and run a SwiftUI app with the real SDK.

**Engineering plan:** design small screens/view-model states for RegTest config,
create/restore/open, addresses/balances, boarding, Ark transfers, Lightning,
history, refresh and exits. Plan loading/errors/retries, lifecycle subscriptions,
task cleanup and Keychain-backed seed storage before writing UI.

**Implement:** add an iOS 16 Xcode project, local SDK dependency, debug networking,
automation identifiers and UI tests. Perform wallet logic through the SDK and
use isolated test app storage; do not log seeds.

**Done when:** the app builds, installs and launches on a simulator, persists and
reopens a real wallet and displays real backend results. Record build/test logs
and screenshots.

### 08 — Build the native Android example

- [ ] Create and run a Kotlin/Compose app with the real SDK.

**Engineering plan:** mirror the iOS workflows with ViewModel/coroutine lifecycle
handling, Keystore-protected seed storage, JNI cleanup and debug host networking.

**Implement:** add the minSdk 24 Gradle app, pinned tooling, generated AAR dependency,
loading/error states, automation identifiers and instrumentation tests. Use host
mapping or adb forwarding, not a developer-specific LAN IP.

**Done when:** the APK builds, installs and launches on an emulator, loads JNI,
and creates/reopens a real wallet connected to RegTest; preserve native/app logs
and test reports.

### 09 — Prepare repeatable simulator/emulator execution

- [ ] Resolve device prerequisites and automate device readiness.

**Engineering plan:** select local/CI OS images; install the missing Android
emulator/system image; define boot readiness, forwarding and test data isolation.

**Implement:** provide bounded setup/test commands. Open local devices through
T3's Device panel and drive them with its returned agent-device launcher; use
platform build/log tools as needed.

**Done when:** both apps execute on actual simulator/emulator processes with
reachable endpoints. Missing runtimes/devices are resolved, not counted as passes.

### 10 — Run end-to-end RegTest acceptance on both apps

- [ ] Demonstrate actual wallet workflows and assertions on iOS and Android.

**Engineering plan:** define deterministic balances/state transitions, block
mining and bounded confirmation waits. Separate complete API contract coverage
from important UI workflows. Identify additional LNURL/BOLT12 fixtures explicitly.

**Implement:** create a wallet on each platform; fund/sync/board; send Ark in both
directions; assert balances/history/notifications; pay and receive Lightning via
CLN/LND; exercise refresh, offboarding and unilateral exit/claim with mined
timelocks. Test restart, restore/rescan, unavailable server, invalid input and
cancellation/reconnect.

**Done when:** real SDK/app outcomes agree with node/server state; restart
preserves data; save transaction IDs, test summaries, screenshots and sanitized
logs. Resolve or explicitly document uncovered scenarios before claiming parity.

### 11 — Add GitHub Actions for Rust and SDK builds

- [ ] Build and test Rust plus generated SDK artifacts on GitHub.

**Engineering plan:** define PR/push triggers, pinned toolchains, Linux/macOS
matrices, caches, API/generation checks and artifact provenance from the same
commit. Separate release/signing permissions from normal tests.

**Implement:** add format/Clippy/tests/audit, strict generation, Apple packaging,
Android AAR/JVM JAR builds, native contract tests and artifact uploads.

**Done when:** actual GitHub workflow runs pass and expose downloadable artifacts
from the migrated commit. YAML validation alone is insufficient.

### 12 — Add GitHub Actions for the apps and runtime tests

- [ ] Build both apps and execute automated simulator/emulator tests in CI.

**Engineering plan:** use macOS/Xcode for iOS and Linux with accelerated Android
emulation. Define RegTest startup and networking explicitly: hosted macOS cannot
be assumed to support Linux Docker service containers. Validate a supported
backend connection/container runtime or provision a suitable macOS Actions runner
before promising an iOS RegTest gate.

**Implement:** build against SDK artifacts from the same workflow, run native app
tests, upload APKs/xcresult/test reports/logs, and execute real RegTest smoke tests.
Run longer Lightning/exit/recovery cases in a defined integration job.

**Done when:** actual Actions runs build both apps and execute tests; required
RegTest jobs have passed, rather than being skipped or mocked. Simulator builds
need no App Store signing credentials.

### 13 — Validate release outputs and finish documentation

- [ ] Deliver reproducible SDKs, apps, RegTest setup, CI and migration guidance.

**Engineering plan:** review parity, platform/test matrices, packaged contents,
API changes, install/build commands and artifact version/checksum metadata.
Distinguish a release-ready build from publishing a release.

**Implement:** document installation, breaking changes, RegTest, app execution
and CI; exercise release-mode packaged consumers and clean builds. Replace or
archive the proof when real tests supersede it; preserve license/attribution.

**Done when:** another developer can reproduce both apps and the backend from
documented commands; every API is accounted for, required runtime/CI checks pass,
packaged consumers work and remaining limitations are accurately recorded.

## Dependency order

00 -> 01 -> 02 -> 03 -> 04 -> 05 -> 06 -> 07/08 -> 09 -> 10 -> 11/12 -> 13.
CI scaffolding and device setup may start earlier after their engineering plans
are written; dependencies and completion evidence still apply.

## Current evidence

The new private GitHub repository exists and the local checkout preserves source
history/license. Earlier proof results are available in experiments/boltffi.
For this plan, Docker responds, Nitro's Compose config parses and iOS device
tooling is available. No RegTest services or mobile apps were started during
planning.

## Completion record — task 00

**Engineering plan and affected paths:** preserve the upstream checkout/history,
then create the requested private GitHub repository and add README.md, AGENTS.md,
this checklist and the existing compatibility evidence.

**Implemented:** created niteshbalusu11/bark-bolt-ffi, configured origin/upstream,
established main and registered the local project in T3. Original reference
checkouts were not changed.

**Validation:** confirmed 14 engineering plans, 14 implementation sections and
14 acceptance criteria; checked local documentation links and staged whitespace;
verified local HEAD and the remote main ref both equal
78768e0a71c6eae15e2f91e2e1d7acb435906a50. GitHub reports PRIVATE visibility and
main as the default branch.

**Artifacts:** https://github.com/niteshbalusu11/bark-bolt-ffi/commit/78768e0a71c6eae15e2f91e2e1d7acb435906a50

**Remaining limits:** all production implementation tasks remain unchecked.
This bootstrap does not claim a working production migration or passing app CI.

## Completion record — task 01

Engineering plan: docs/plans/01-baseline-and-api.md.
Implemented: docs/api-baseline.json records all 128 exports, 40 records, 14 enums
and two callback traits with original names/signatures.
Validation: all 52 original native tests passed; native Clippy with warnings
denied passed under Rust 1.90 after adding installed Homebrew tools to PATH.
The first attempt failed only because protoc was outside PATH.
Remaining limits: migrated signatures and host parity are validated in task 03/05.
