# Noah native API parity

Reference: Nitro Ark v0.0.144, commit 10bcf3ec9325696601de10f833fab747f0a5ec4c.
Keep Bark bfbe6b7c and BoltFFI 0.31.1 pinned. Preserve licenses and upstream
wallet/persistence/transaction semantics. P2 is deferred for a product decision.

## P0 engineering plan (before implementation)

Affected paths: src/functions.rs, new src/crypto.rs and src/backup.rs,
src/ffi/{db,wallet,mod}.rs, new src/core/state_changes.rs, src/types.rs,
Cargo.{toml,lock}, sdk/swift-support/Concurrency.swift, tests/{swift,kotlin},
tests/fixtures, docs/api-baseline.json, scripts/check-api.py, docs/migration.md.

Keys: wallet methods delegate to Bark peek/derive-store, never exporting secrets.
Offline methods copy Nitro's m/350/index derivation; the u32 high bit selects a
hardened child. Message signatures use Bitcoin signed_msg_hash and deterministic
secp256k1 ECDSA DER. Check fixed public mnemonic vectors for both paths, 0/1/7
and high-bit boundaries, all networks, failures and tampering in all three hosts.

Snapshots: retain the actual selected SQLite path on each wallet. Track live
paths process-wide with RAII registrations to reject validation of live files,
including hard links. Serialize creation; Online Backup copies 128 pages per
step with 10ms pauses and 30s continuous-busy timeout. Inspect only temporary
copies, check integrity/FKs/schema/typed Bark data, preserve source bytes, fsync
then publish without overwrite. Accept Nitro schema 42 using a real old-schema
fixture. Identity uses wallet properties, including server mailbox key. Validation
is independent of loaded handles and never infers expected identity implicitly.

State changes: independent read-only connection polling data_version at 750ms;
initial/databaseChanged/resyncRequired and monotonic sequence. A holder owns the
worker; explicit stop/drop closes it. One pending receive, reusable cancellation,
foreign task cancellation drops the wait without leaving the slot held. No
wallet transaction is changed by the observer.

Validation: cargo fmt, cargo test, warnings-denied Clippy; strict generation;
API inventory check; freshly built Swift contracts and compiled/executed Kotlin
contracts. Real SQLite mutation, rollback, WAL, alias, busy and corruption tests.
Record actual commands/outcomes below before marking task complete.

## P1 engineering plan (refine before implementation)

Affected paths: core/ffi wallet and onchain, functions/types, both app callers,
both contract suites, baseline/checker and migration guide. Delegate transaction
building, rounds, metadata patching, exits and mailbox signing to pinned Bark.
Structured destinations carry u64 sats; boarding returns every ID. Drain uses BDK
drain selection. Pure parsers do not load a wallet or contact a server. Expose
only available Lightning details, never private keys or the P2 payment secret.
Document fee units and absolute expiry. Validate Rust and both host contracts,
build apps after changed result shapes, and run live RegTest for transactions.

## Release engineering plan

One PR per P0/P1 group with small commits. After acceptance, bump to a fresh
version, build full Apple and Android ABI matrices, stage generated bindings and
SPM checksum via existing scripts, create tag/release with XCFramework/AAR/JAR,
SHA256SUMS and manifest. Verify downloaded artifacts and clean consumers; record
actual hosted CI results separately from local results. Never overwrite v0.1.0.

## Actual outcomes and remaining limits

P0 implemented: wallet and offline public-key/signature exports; safe SQLite
snapshot export/validation and schema42 fixture; 750ms committed-state observer
with foreign/explicit cancellation, stop and reuse; existing-wallet missing-file
preflight. API checker includes the new exports, records and field names.

P0 validation (2026-10-19): `BARK_REGTEST=1 cargo test --locked` passes 66
Rust tests including actual wallet operations against the existing isolated
RegTest backend (ports 3635/19443). `cargo clippy --locked --all-targets -- -D
warnings` and formatting pass. Fresh Apple/Android local-overlay SDK builds use
strict generation. `BARK_REGTEST=1 swift test --package-path tests/swift` passes
6 tests. `BARK_REGTEST=1 ./gradlew :sdk:jvm:test :sdk:jvm:jar
:sdk:jvm:testPackaged` compiles Kotlin and passes 6 tests in each mode. Golden
vectors cover all four exposed networks and hardened boundaries. API parity
passes 141 exports, 44 records, 14 enums and 2 callbacks. No generated files were
patched. Full ABI/release/hosted CI acceptance belongs to the release task.

Remaining P0 limits: no actual end-user `.noahbackup` archive was supplied;
validation covers a matching schema42 plaintext fixture and real newly exported
wallet snapshots. No physical devices were used for these contract tests.
P1 and release acceptance are pending.
