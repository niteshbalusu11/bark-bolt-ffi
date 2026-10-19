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

Refinement before P1 code: keep sentToAddresses/receivedOnAddresses names but
replace their elements with MovementDestination { paymentMethod: PaymentMethod
{ type, value }, amountSats }. Metadata patches must be JSON objects <=16KiB,
matching Nitro; Bark performs the merge. PendingBoard gains vtxoIds and drops
the lossy singular field. Pure decode returns state=Unknown and registered=false
because those are local DB facts; wallet VTXOs retain their stored state.
Anchor/point are available on both full and Bare VTXOs, so all wallet views
expose them without fetching genesis.
Round status uses a typed outcome with funding txid/error and terminal flags;
the current pin has no unsigned funding txid status, so its list is empty.
Exit packages include parent/child raw hex and their statuses without another
network lookup. Absolute mailbox expiry gets a distinct Rust export (Rust has
no overloads); document the generated host name. Drain retains network
validation; ownership queries validate syntax and inspect scriptPubkey, matching
Nitro regardless of the address network. Callback backends cannot drain.

## Release engineering plan

One PR per P0/P1 group with small commits. After acceptance, bump to a fresh
version, build full Apple and Android ABI matrices, stage generated bindings and
SPM checksum via existing scripts, create tag/release with XCFramework/AAR/JAR,
SHA256SUMS and manifest. Verify downloaded artifacts and clean consumers; record
actual hosted CI results separately from local results. Never overwrite v0.1.0.

## Actual outcomes and remaining limits

Swift stack regression plan: the minimal live test
`swift test --package-path tests/swift --filter testWalletSyncOnSwiftWorkerStack`
reproduces SIGBUS on the Swift worker stack in Wallet::run. Box Bark's large
future before wrapping it to retain the live DB registration; keep ownership
and cancellation semantics unchanged. Re-run the exact reproducer and full
funded Swift/Kotlin contracts before recording a fix. Backport this P0 fix to
its PR as well as P1.

CI refinement: after the isolated stack starts, the hosted Android job also
runs Rust's live contracts. The paired integration workflow starts its backend
before contracts and sets BARK_REGTEST=1 for Swift/Kotlin, so live host APIs are
executed in addition to the app flows. No existing volumes are reset.

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

## P1 actual outcomes (before independent review)

Implemented all P1 exports and DTO fields; absolute mailbox host overloads are
handwritten support files calling regenerated Rust bindings. No generated code
is patched. Pure decoding uses Unknown local state. All boarding IDs and exit
packages are retained. Pending round conversions cover every current outcome.
The original derivation/SQLite schema and upstream transaction logic remain.

Validation: full BARK_REGTEST=1 Rust suite passes 71 tests; warnings-denied
Clippy passes. Swift compiles and passes 9 contracts including funded boarding,
Ark receive/destination amount sums, metadata patch, BDK drain, exits/packages,
destructive drop in a disposable wallet, absolute mailbox auth, and invoice
details. Kotlin compiles and passes 8 contracts normally and from its actual
JAR, including the same funded cases; the Android AAR and example APK build.
The Swift stack reproducer failed with SIGBUS before boxing the inner future,
then passed in 0.139s; all 9 Swift contracts pass after the fix. The fix is also
backported to P0 PR #1 (357d773). Mailbox Schnorr auth is randomized, so its tests
assert identity/expiry rather than byte equality across distinct signatures.
Deterministic ECDSA golden signatures still compare exact bytes in all hosts.
Eight Python tooling tests and strict API parity (149 exports, 50 records, 15
enums, 2 callbacks) pass. Local full ABI release/hosted CI remain release gates.

Remaining limits: terminal round outcomes have Rust conversion tests; empty
pending-state calls run in both hosts. The pinned upstream does not expose
unsigned round funding IDs or retain settled Lightning HTLC/movement details.
P2 stays deferred. Independent Fable 5.1 High review and release are pending.

## Fable 5.1 High review: validation and repair plan

Review round 1 examined combined PR head 249386e, without executing tests.
Before changing behavior, reproduce its observer lock-contention finding with
a real rollback-journal SQLite writer holding EXCLUSIVE across a poll tick.
In src/core/state_changes.rs, retry BUSY/LOCKED on the next 750ms tick, retaining
the short busy timeout for responsive stop; other errors still emit
resyncRequired and terminate. Verify committed changes arrive after unlock and
the observer remains active. Preserve Nitro's queued-event drain after stop,
document it and test it rather than discard already committed notifications.

For src/backup.rs, reproduce validation silently ignoring an uncheckpointed WAL.
Validation must only read the input, so reject nonempty -wal/-journal siblings
with an actionable error instead of opening/recovering the source with SQLite.
Check before and after the raw copy, and prove source/sidecar bytes are unchanged.
Checkpointed createSnapshot outputs and extracted schema42 fixtures must still
validate. Update docs/migration.md with this self-contained snapshot boundary.

Keep the specified 10ms pause between backup steps and continuous-busy 30s
timeout; an overall deadline would change the requested contract. Keep Bark's
and Nitro's fee-rate acceptance, including zero, rather than introduce a new
transaction-validation policy. Record the reasoning in the review dispositions.
Backport these P0 repairs to PR #1 and merge into PR #2. Run Rust tests/Clippy,
regenerate both full release SDKs, execute Swift and Kotlin packaged contracts,
and verify release provenance after the final changes.

Actual repair outcomes: the lock regression first failed with resyncRequired;
the WAL regression first failed because validation returned stale main-file
information. After the fixes, BARK_REGTEST=1 cargo test --locked passes all 74
tests and cargo clippy --locked --all-targets -- -D warnings passes. The stop
queue regression passes with Nitro's documented drain behavior. Full release
SDKs are rebuilding from the repaired source before host contracts run again.

Release refinement before CI packaging changes: have the Android CI Gradle call
generate both Maven POMs and upload them with the AAR/JVM JAR. Consume the actual
Linux CI-generated POM rather than editing host-specific generated metadata.
After final green checks, merge P0 then P1 as the user requested, and tag the
merged main commit. Verify both the published checksums and clean Git consumer.

Additional parity correction plan: Nitro v0.0.144 onchain.rs:24 uses
assume_checked().script_pubkey() for is_mine, so a valid address from another
network must be queried rather than rejected. In core/ffi onchain, parse syntax
without require_network for this read-only query. Keep send/drain validation.
Add a Rust regression re-encoding an owned RegTest script as a Bitcoin address
and checking true, plus valid foreign-network non-owned addresses in Swift and
Kotlin. First reproduce the network error, then run the full contracts and
rebuild/re-stage release artifacts from the corrected source.

Actual ownership correction outcome: the new Rust regression first failed with
"bitcoin address is not valid on regtest". With syntax-only parsing for isMine,
all 74 live Rust tests pass, including ownership of the same script encoded on
Bitcoin and false for an unrelated valid Bitcoin address. Swift/Kotlin contracts
include the foreign-network query; final rebuilt SDK execution remains a gate.

Final local release outcomes: full optimized Apple five-ABI and Android
three-ABI plus Darwin JNI builds pass. Swift artifact staging/check-source
passes and compares slices with current Cargo output/deployment targets.
Final live Swift contracts pass 9/9; final live Kotlin contracts pass 8/8
normally and 8/8 from the actual JAR, including the ownership correction.
AAR/JAR ABI/class checks and the 149-export API inventory pass. Both Maven
POMs are generated, iOS rebuild passes, and Android debug/release APKs build
successfully. Hosted CI/merge/publication/download acceptance remain pending.

Post-publication consumer validation plan: verify fresh downloaded checksums and
run the live Swift contracts through a clean public Git consumer. The first run
compiled and passed offline vectors/SQLite parsing, but the owned RegTest stack
had stopped during CI waiting (RPC connection refused). Restart only project
bark-bolt-ffi with its existing volumes; never stop/reset the separate scripts
stack. Its CLN then reported stored height 980 ahead of Bitcoin at 681. Confirmed
both are on bark-bolt-ffi_default with project-specific volumes and CLN RPC
bitcoind:18443. Advance only this isolated test chain to 981, restart its existing
services and re-run the same consumer. Do not delete databases or volumes.

Final completion outcomes (2026-10-19): both P0/P1 PRs pass every hosted
Rust/Apple/Android job and are merged. The merged main tree b8ca615 exactly
matches candidate 8428f35; Swift check-source passes. Published immutable tag
v0.2.0 at b8ca615 with eleven assets, including full XCFramework/AAR/JAR/POM,
manifest, provenance/build info and checksums. All ten checksummed files match
fresh release downloads. A clean public Git consumer passes all nine live Swift
contracts through the published binary. The initial live check failed solely
with RPC connection refused after the owned backend stopped; its persisted
captaind index was 1010, so the isolated chain was advanced to 1011 and services
restarted without database/volume resets. The identical consumer passed after
recovery. Both this stack and the separate scripts-* stack remain running.
Tag Rust/provenance checks pass; extra tag platform rebuilds were still running
at record time, while the complete equivalent-source PR CI matrix is green.
Task 17 acceptance is complete; physical/minimum-OS runtime and an actual
end-user backup archive remain the previously stated limits, with P2 deferred.
