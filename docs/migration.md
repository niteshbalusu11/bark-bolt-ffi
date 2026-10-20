# Installing and migrating

This SDK exposes Bark master `bfbe6b7c3088a09dd4e81c4ac24cdc763a627a8c`
(0.7.1-dev,2026-10-16) through BoltFFI 0.31.1. Swift and Kotlin are the
supported consumer languages. The Rust core remains in this repository.

## Swift

Add `https://github.com/smolcars/bark-bolt-ffi` in Xcode's package dependencies,
select version 0.4.0, and link the `BarkBoltFFI` product. In a package manifest:

```swift
.package(url: "https://github.com/smolcars/bark-bolt-ffi.git", exact: "0.4.0")
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

Use `mavenLocal()` with `io.github.niteshbalusu11:bark-bolt-ffi-android:0.4.0`,
or the host-specific `bark-bolt-ffi-jvm-darwin-arm64` / `bark-bolt-ffi-jvm-linux-x86_64`
artifact. Import `io.github.niteshbalusu11.bark.*`. The Maven POM supplies the
coroutines dependency. The Android AAR embeds all three JNI ABIs and consumer
R8 rules. JVM JARs contain the native library for the build host; choose a JAR
matching the deployment OS/architecture. Linux CI builds on Ubuntu24.04; rebuild
locally when targeting an older libc. Maven Central publication is separate.
Versioned AAR/JAR/POM files are attached to the
[v0.4.0 release](https://github.com/smolcars/bark-bolt-ffi/releases/tag/v0.4.0);
verify them against its SHA256SUMS. v0.4.0 attaches the Linux x86_64 JVM JAR;
build the Darwin JVM JAR from source. Use matching v0.4.0 bindings/native binaries,
including on Swift, because the typed error record changes the ABI. GitHub Actions also uploads artifacts and
example APKs. To use a downloaded AAR or JVM JAR directly, include it as a local
file dependency and declare `org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2`
explicitly; local file dependencies do not resolve the attached Maven POM.

## Breaking API and ownership changes

| Area | New contract |
| --- | --- |
| Swift module | `BarkBoltFFI` |
| Kotlin namespace | `io.github.niteshbalusu11.bark` |
| Errors | `BarkError.code` classifies supported failures and `message` retains the complete Rust error chain; update catches that matched the old UniFFI enum. Unexpected callback exceptions are converted to this error. |
| Factories | BoltFFI uses concrete object factories. Swift `Wallet.openWithOnchain(...)`, Kotlin `Wallet.openWithOnchain(...)` construct a wallet with an existing onchain handle. |
| Open arguments | `WalletOpenArgs` contains values only. Pass the onchain object separately to `openWithOnchain`; `open` supports wallets without one. |
| User agent | Set `Config.userAgent` explicitly to send a custom value. BoltFFI does not infer a Swift/Kotlin user-agent from build-time language variables. |
| Defaults | Optional config fields use nil/null defaults. Check generated declarations for unsigned integer and enum spellings. |
| Kotlin resources | Object wrappers implement `close()`. Keep them alive while needed and close notification/onchain/wallet wrappers during teardown. In-flight native work retains its own references. Calling a closed wrapper fails. |
| Swift resources | ARC releases native handles. The separate concurrency adapter marks synchronized native object wrappers Sendable for Swift 6 actor crossing. |
| Callbacks | Implement all methods of `CustomOnchainWalletCallbacks` or `BarkLogger`. Native workers can call them off the UI thread; synchronize mutable state. Native ownership retains callbacks until their owner/logger registration is released. |
| Notifications | One pending receive per holder. Explicit cancellation or host-task cancellation releases that wait; reuse is supported. Concurrent waits return an error. |
| Recovery outcome | `open` mirrors upstream best-effort recovery. Inspect `recoveryStatus()` and the completed report's `isComplete` before declaring seed recovery successful. |
| Cancellation | Cancelling a foreign await does not roll back a submitted wallet operation. Tokio work already submitted may finish; sync/reconcile the wallet before retrying a payment. |
| Daemon | Stop with `stopDaemonWait()` before replacing a wallet. Dropping the last native wallet also stops its daemon. |

All128 original callable exports, 40 records, 14 enums and both callback traits
are tracked in `api-baseline.json` and checked against strict host generation.
v0.3.0 brings the checked inventory to 150 exports, 50 records and 15 enums.
Generated files are never patched; the Swift concurrency adapter is handwritten
in `sdk/swift-support`. There is no UniFFI runtime, JNA or browser product.

SQLite uses `db.sqlite`, with compatibility for the older `bark.sqlite` filename. Open an existing wallet with
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
| Android | API 24 | arm64-v8a/armeabi-v7a/x86_64 | API 35 ARM64 locally and x86_64 in CI |
| Kotlin/JVM | Java17 | build-host native JAR | macOS ARM64 and Linux x86_64 contracts |

Minimum OS versions and the other packaged ABIs are compile targets; this does
not claim physical-device or every-OS runtime testing. The example apps are
RegTest tools with debug-only local HTTP networking and test command bridges.
They are not store-distributed wallet products.

Live acceptance covers boarding, two-way Ark transfers, BOLT11 send/receive,
notifications, refresh, offboarding, unilateral exits/claims, restart and funded
seed recovery. LNURL/LNAddress and BOLT12 exports remain available and retain
upstream validation, but no external provider acceptance fixture was run.
See [v0.3.0 validation](validation/2026-10-19-v030.md) for full-balance/opening contracts and release hashes,
`validation/2026-10-19-noah.md` for previous Noah parity results and CI links,
and `validation/2026-10-19-master.md` for the previous master release;
`validation/2026-10-18.md` retains prior release 0.7.1 evidence.

## Developing and preparing a release

The root SwiftPM package and `sdk/swift` describe the last published release.
Development source changes build fresh SDKs in `dist/`; normal PR/main CI checks
the integrity of the committed release files without requiring them to match
the newer Rust source. Tag CI additionally checks source/version provenance.

For a new release, bump `[package].version` in Cargo.toml first; Gradle reads that
version for SDK publications and the Android example. Build the full optimized
Apple SDK, then run `python3 scripts/swift-artifact.py --stage-release` and
`python3 scripts/swift-artifact.py --check-source`. Staging checks local/remote
tags and refuses an already-published version before writing anything. The ZIP
uses fixed timestamps/permissions, so unchanged input bytes produce the same
archive despite file mtime changes. Publish the new archive at its versioned URL
and run the clean Git consumer. Never replace a published version's binary.

Both example restore paths scan onchain from height 0. Each automated restore
uses a new database and checks a completed recovery report. Re-entering the
same phrase is allowed; a different phrase cannot replace an existing seed.
Seeds stay persisted before wallet opening so a failed open can retry safely.
iOS wallet directories and Android wallet files/encrypted seed preferences are
excluded from backup/device transfer because their seed protection is bound to
the original device. See [Android's backup behavior](https://developer.android.com/about/versions/12/behavior-changes-12#backup-restore).

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
`niteshbalusu/captaind:nightly-2026-10-19` (same master commit), each pinned by digest.
The server requires Bitcoin31, which the stack pins. Both images are pinned to the exact master revision; the older captaind tag
lacks required recovery RPCs. The dates are registry UTC
build dates; the Bark image was published on October 18 in America/New_York.

Android consumers should restrict `defaultConfig.ndk.abiFilters` to
`arm64-v8a`, `armeabi-v7a`, and `x86_64`, as the example does. This prevents a
transitive dependency's x86 library from advertising an ABI without Bark JNI.

## Noah native migration (Nitro Ark v0.0.144)

Amounts are unsigned 64-bit satoshis. Fee rates use these explicit units:

| API | Unit |
| --- | --- |
| `Config.fallbackFeeRate` | sat/kwu; Nitro sat/kvB `10000` becomes `2500` |
| `OnchainWallet.feeRates()` (`fast`, `regular`, `slow`) | sat/kwu |
| Onchain `send`, `drain`, custom-wallet transaction callbacks | sat/vB |
| Emergency-exit estimates and `progressExits` | sat/vB |

To open an **existing** Noah wallet, use its original mnemonic, network and
folder with `OnchainWallet.openExisting(network:mnemonic:config:datadir:)` in
Swift, or `OnchainWallet.openExisting(network, mnemonic, config, datadir)` in
Kotlin. Then call `Wallet.openWithOnchain` with `createIfNotExists = false`.
Both existing-wallet paths validate `bark_properties`, network and Bark's seed
fingerprint before opening a migrating SQLite connection or creating BDK tables.
Missing, empty, foreign, wrong-network and wrong-mnemonic databases fail without
changing database bytes, tables or directory entries. Schema 42 remains supported;
normal migration/BDK loading happens after identity validation. `Wallet.open`
without an onchain handle applies the same preflight when creation is disabled.

`OnchainWallet.default` remains the creation-capable factory. Use it when creating
or recovering into a new directory; calling it before the existing-wallet opener
would already allow writes. `createIfNotExists = true` retains its creation
behavior. Older `bark.sqlite` files retain compatibility filename selection.
Noah's application-level read-only check can remain as defence in depth.
The SDK never recovers a hot rollback journal or creates a missing WAL index
during preflight: files requiring that recovery fail before any writes. Existing
WAL/index pairs, including zero-length WALs, are read with SQLite's read-only connection; closed WAL databases use
an immutable connection to avoid creating sidecars. As with ordinary wallet open,
the app must serialize opening against foreign database writers and file
replacement.

## Wallet balance in v0.3.0

`Wallet.balance()` exposes all nine non-overlapping Bark buckets, in satoshis.
Show `spendableSats` as available to pay and `totalSats` as the offchain wallet's
full balance. `pendingSats` and `totalSats` come directly from Bark's `pending()`
and `total()` methods. `totalSats = spendableSats + needsRefreshSats + pendingSats`.
Do not construct the total using the six fields exposed before v0.3.0.

| Field | Meaning |
| --- | --- |
| `spendableSats` | VTXOs available to spend now. |
| `needsRefreshSats` | Unlocked VTXOs that have expired or reached the server's exit-depth limit. Kept out of spendable until refreshed; still part of total. Bark may offboard or exit them. This differs from the proactive maintenance refresh-expiry threshold. |
| `pendingLightningSendSats` | Locked outgoing Lightning inputs until settlement or failure. |
| `claimableLightningReceiveSats` | Incoming HTLC VTXOs whose preimage has been revealed but whose payment has not yet been swapped into spendable VTXOs. Included in pending. Still-cancellable incoming payments are not counted. |
| `pendingInRoundSats` | Inputs locked by an active round. A participation waiting without a lock remains spendable. |
| `pendingExitSats` | Exits committed onchain but not yet claimed into the onchain wallet. Cancellable exits are not counted here. |
| `pendingBoardSats` | Broadcast board VTXOs awaiting enough confirmations for registration. Unbroadcast boards are not counted. |
| `pendingArkoorSendSats` | Whole inputs locked in an outgoing Ark send, including change until finalization. |
| `pendingOffboardSats` | Whole inputs locked before offboard broadcast, including change. After broadcast, those funds belong to the onchain wallet. |

`pendingSats` includes all seven operation buckets; it excludes spendable and
needs-refresh funds. Each VTXO is counted once. When the daemon is disabled,
sync before querying balances. All prior field names remain available; regenerate
consumers against the v0.3.0 SDK because the native record layout has changed.

Wallet `peekPublicKey`, `deriveStoreNextKeypair` and `signMessage` use Bark's
`m/350'/0'/index` keychain. Peeking/signing an unstored index fails. Deriving the
next key persists it and returns only its compressed public key and index.
Top-level `derivePublicKeyFromMnemonic` and `signMessageWithMnemonic` work offline
and use Nitro's separate `m/350/index` path with an empty BIP39 passphrase. These
keys deliberately differ from wallet keys. For either path, `u32` indices with
bit 31 set become hardened children: `2147483648` means `0'`, `4294967295` means
`2147483647'`. The purpose `350` in the offline path is never hardened.
Signatures are deterministic ECDSA over Bitcoin's `signed_msg_hash(message)`,
encoded as lowercase DER hex. `verifyMessage` returns false for mismatched valid
signatures, and throws for malformed DER/public-key hex. No API exports private
keys. The same public keys/signatures result on all supported networks.

`createSnapshot(destinationPath)` exports plaintext SQLite suitable as the
`db.sqlite` member of a Noah v2 backup. It serializes creation process-wide,
uses SQLite Online Backup (128 pages/step, 10ms pause, 30s continuous-busy limit),
checks integrity/foreign keys, fsyncs and atomically publishes without overwrite.
The SHA256 is lowercase hex over the plaintext database; `mailboxPubkey` is the
server mailbox key. Encrypt/package the resulting file in the app's backup flow.
Top-level `validateWalletSnapshot(path, expected)` works without an open wallet,
rejects live DB aliases, reads the input into a private temporary copy beside it,
checks schema/integrity/FKs, migrates that copy and loads typed Bark/BDK state.
It supports Nitro's schema 42. The input directory must be writable for temporary
validation files. The input bytes remain unchanged; expected identity is optional
and explicit. The API validates an extracted database, not a `.noahbackup` ZIP.
The input must be self-contained: nonempty `-wal` or `-journal` sidecars are
rejected rather than silently omitted or recovered by mutating the input.
Export with `createSnapshot` before packaging a live database. The 30s timeout
counts continuous busy/locked time, not total elapsed backup time.

`wallet.stateChanges()` returns an independent `StateChangeSubscription`. Receive
with `nextChange()` in a loop; reasons match Nitro: `initial`, `databaseChanged`,
`resyncRequired`, with monotonic sequence numbers. It polls `PRAGMA data_version`
on an independent read-only connection every 750ms, so commits can coalesce and
rollbacks do not emit changes. Temporary SQLite busy/locked polls retry on the
next tick. Treat `resyncRequired` as a request for a full
state refresh and resubscribe. Only one pending wait per holder is allowed;
`cancelNextChangeWait()` returns nil/null for that wait and permits reuse. Host
task cancellation also releases the wait. `stop()` permanently stops observation,
unblocks receivers and sets `isActive()` false. Events already queued still drain
before nil/null, matching Nitro; close Kotlin holders on teardown.
Use these notifications to schedule backups after any committed state change.

Noah P1 API changes:

- Movement `sentToAddresses` / `receivedOnAddresses` now contain
  `MovementDestination { paymentMethod: { methodType, value }, amountSats }`.
  Sum `amountSats` directly; payment methods are no longer JSON strings.
  `updateHistoryMetadata(movementId, patchJson)` accepts a JSON object up to
  16KiB in UTF-8 and delegates merging to Bark.
- `OnchainWallet.isMine(address)` checks ownership. `drain(address, feeRateSatPerVb?)`
  uses BDK's actual drain and returns its txid; omit the sat/vB rate for the
  chain's regular rate. Drain validates the configured address network;
  `isMine` accepts any valid address network and queries its script, as in Nitro. Custom
  callback backends support ownership via scriptPubkey but have no configured
  network and do not implement `drain`.
- `decodeVtxo(hexOrBase64)` is pure. All VTXOs expose `serverPubkey`, `exitDelta`
  (blocks), `anchorPoint` and `point` (structured txid/vout). Pure decoding sets
  state to `Unknown` and registered to false because these local DB facts are
  not encoded. `dangerousDropVtxo(id)` deliberately invokes Bark's destructive
  recovery operation; use it only when forgetting that VTXO is intended.
- `syncPendingRounds()` advances pending rounds and reports `PendingRoundStatus`
  sorted by round ID, including the `RoundOutcome`, funding txid, error,
  `isFinal` and `isSuccess`. On this Bark revision `unsignedFundingTxids` is
  empty: the upstream status enum does not carry unsigned funding transactions.
  Capture terminal results before they disappear from pending-round listings.
- `boardAmount` / `boardAll` return `PendingBoard.vtxoIds`, containing every
  created VTXO ID. The singular `vtxoId` field is removed.
- `getExitStatus(..., includeTransactions: true)` now returns transaction
  packages, with parent/child txids and raw hex, child origin, and the tracked
  `ExitTxStatus` while the current state is `Processing`. Outside Processing,
  package status is absent; use the enclosing exit state. Omit transactions for
  a lightweight status; its transaction list is empty.
- `mailboxAuthorization(authorizationExpiry: Int64/Long)` is a handwritten host
  overload for an **absolute Unix timestamp in seconds**. It calls generated
  `mailboxAuthorizationAt`, returning `{ mailboxId, expiry, encoded }` with the
  actual absolute expiry and hex-encoded authorization. The existing
  `mailboxAuthorization(expirySecs: UInt32/UInt)` keeps its relative-seconds
  contract and returns encoded hex. Kotlin consumers import the overload from
  `io.github.niteshbalusu11.bark` (the wildcard import includes it).
- `ArkInfo.mailboxPubkey` is the server mailbox public key.
  `parseArkAddress(address)` returns user pubkey, Ark ID and test-network flag
  without network access. In-progress Lightning sends/receives expose HTLC IDs
  and movement ID when Bark has created them, alongside invoice and amount.
  Settled receives retain invoice/amount but their upstream fact table does not
  retain HTLC IDs/movement ID; consult history for those. A settled send's
  upstream fact contains only payment hash and preimage, as before.

P2 trusted/untrusted pending balances and BOLT11 payment-secret exposure remain
subject to a product decision.


## Prepared onchain sends (v0.4.0)

Call `OnchainWallet.prepareSend(address, amount, feeRateSatPerVb, confirmedOnly)`
with `OnchainSendAmount.Amount(amountSats)` or `DrainAll` (use the generated
Swift/Kotlin case spelling). Show the returned `amountSats`, `feeSats`, `vsize`
and `inputs` for review. The fee comes from the signed transaction's actual
selected inputs. Preparation does not persist or broadcast a transaction.

`broadcastPrepared(id)` records and submits those exact signed bytes, returning
`BroadcastResult { txid, broadcast }`. `accepted` means the chain source accepted
submission; `failed(reason)` means submission returned an error and the stored
transaction remains eligible for sync retry. A failed RPC may have reached the
node before its response was lost. `notAttempted(reason)` means persistence
failed before submission; no transaction was inserted into the live graph.
Retry uses the same bytes and checks for competing spends/reservations again.

`discardPrepared(id)` releases an unbroadcast preparation. After a broadcast
attempt it only forgets the handle; it cannot cancel the stored spend or its
future sync retries. IDs/reservations are process-local and survive while any
SDK handle or attached daemon retains the shared onchain wallet; prepare again
after restart. Cancelled preparation waits discard an unclaimed result.
Broadcast waits retain the existing submitted-payment cancellation semantics.
Callback-backed onchain wallets return `unsupportedOperation` for this API.

Reservations exclude inputs from legacy sends, drains, boarding and exit CPFP.
Whole boarding calls serialize with preparation; pending board checkpoints from
all attached Ark databases exclude their funding inputs. Opening another SDK
handle for the same canonical SQLite path shares reservations and validates
network/mnemonic identity. Confirmed-only selection also applies to drains.

`BarkError.code` exposes typed invalid-address/amount/fee-rate, dust,
insufficient-funds, unavailable-input and unknown-preparation failures. The
message retains the complete cause. Unclassified upstream errors use `other`;
existing message-only constructors default to that code.

`Wallet.pendingLightningSends()` already supplies each checkpoint's
`paymentHash`, `amountSats`, `feeSats`, invoice and HTLC details. Before resuming
with `checkLightningPayment`, match by payment hash and compare `amountSats`
(and the fee presented) with the reviewed payment. A matching hash alone does
not establish that an amountless invoice checkpoint has the reviewed amount.
