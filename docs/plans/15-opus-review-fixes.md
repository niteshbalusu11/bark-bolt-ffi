# Opus 5.5 review: validation and fixes

Review: original d3c0f38 through 863326a; model claude-opus-5-5. Validate each
finding independently before implementation. Preserve v0.1.0 tag/assets and
all wallet databases/seeds. No hand edits to generated Swift/Kotlin files.

## 1. Release metadata and development CI (confirmed)

Paths: scripts/swift-artifact.py, Python regression tests, root/module Gradle
files, check-kotlin-artifacts.py, ci.yml, migration guide. Normal verification
must validate committed release files/manifest without requiring development
Rust source to equal the last published release. Add explicit strict source
verification for release preparation. Read SDK version from Cargo metadata;
refuse staging if its tag exists locally or remotely, before mutating artifacts.
Use fixed ZIP metadata and streamed copies so file mtimes do not alter checksums.
Gradle publications/app version read the Cargo package version, not copied literals.
Root SwiftPM remains the immutable published SDK; local SDKs use dist/.

Tests before fixes: comment-only Rust edit must pass normal verification but fail
strict verification; modified committed bindings/manifest must fail both; an
existing remote tag must reject staging before any output writes; identical
bytes with different mtimes must produce identical ZIPs. Run these in CI.

## 2. Developer-host runner (conditional risk, hardening warranted)

Repository API currently returns approval_policy=all_external_contributors;
the review's no-approval exploit condition is absent. Environment filtering alone
cannot isolate a job from its OS user, Keychain or Docker. Do not claim otherwise.

Paths: ci-regtest-runner.py, integration.yml, docs/devices.md and runner tests.
Require the strict fork approval policy before registration. Register a fresh,
unique per-run label and dispatch only integration.yml on main with that label,
instead of accepting the first job on a shared static label. Run the listener
with an explicit tool/device environment, keeping parent GH/SSH/token variables
out. Keep registration/cleanup authentication in the parent. Restrict the job to
manual main dispatch, disable persisted checkout credentials, and remove guidance
for a permanent developer-machine runner. Document trusted code only; recommend
an isolated runner account/machine and workflow-restricted groups where available.
No organization-wide or repository policy mutations are needed.

Tests: unsafe policy fails before registration; child environment excludes sample
GH/SSH credentials; workflow/main/unique-label dispatch arguments are exercised
with a focused fake gh boundary. Then run a real manual paired RegTest job using
the updated helper; verify runner and per-run volumes are removed.

## 3. User agent and stale public documentation (clarification/confirmed)

The removed language-specific user-agent helper depended on build-time variables;
breaking changes were accepted. Keep optional Config.userAgent semantics and
document that no host-language user agent is inferred. Correct run_daemon's
reference to the removed WalletOpenArgs.onchain field to open_with_onchain.
Regenerate development host docs; keep published v0.1.0 files unchanged.

## 4. Restore behavior and acceptance (confirmed test gap)

Paths: both WalletModels, SeedStores, app-regtest.py and native tests. Initial
onchain scanning must follow an explicitly supplied restore phrase, including
retry after an earlier open failure. The debug path must call that same open
logic, and each restore assertion gets a distinct new database. Assert recovery
status/isComplete and empty movement history where appropriate; verify wallet
fingerprint/funded Ark/onchain balances against the original. No recovery phrase
may appear in command arguments, test output or logs.

Keep the seed stored before open: a partially successful open must retain its
seed for safe retry. Do not adopt the proposed delayed seed write or destructive
forget-wallet action. Allow re-entering the same stored phrase; reject a different
phrase explicitly even when the model already cached a seed. This preserves the
existing wallet and avoids silently ignoring changed restore input.

## 5. Backup consistency (confirmed platform gap)

Paths: iOS wallet directory setup; Android manifest/dataExtractionRules resource.
Device-bound seed keys cannot travel with backed-up databases/encrypted prefs.
Exclude the iOS wallet support directory and Android wallet files/seed preferences
from backup/device transfer. Do not delete existing data when key access fails.
Validate iOS exclusion metadata on simulator wallet directories and Android
compiled manifest/rules; compile/lint both apps. Actual cross-device transfers
remain outside available-device coverage.

Sources: https://developer.android.com/about/versions/12/behavior-changes-12#backup-restore
and https://developer.apple.com/documentation/foundation/urlresourcevalues/isexcludedfrombackup

## 6. Focused Rust cleanup (confirmed)

Paths: src/ffi/wallet.rs, src/core/wallet.rs, src/ffi/onchain.rs, Cargo files.
Replace repeated clone/offload boilerplate with one private generic helper using
shared Arc core handles and Send+'static futures. Keep exported async signatures,
Tokio offload and foreign-cancellation behavior. Remove duplicate inner handle,
unused from_inner/dead-code suppression, obsolete TODO, unused direct invoice
dependency and unused direct Tokio signal/fs features. Keep directly used time.
The synchronous config getter can return within its existing async API without
spawning Tokio work. No upstream wallet behavior or validation changes.

Validation: fmt, warnings-denied Clippy, Rust tests/audit; strict Swift/Kotlin
regeneration/API parity; packaged contracts including real-wallet cancellation,
config forwarding and reopen; both rebuilt apps, native UI tests and fresh full
RegTest acceptance. Push changes and verify actual hosted CI. Record findings,
rejected suggestions, commands and results before completing task15.

Gradle version-centralization check caught an SDK/app capability collision: both
leaf projects are named android. Keep the example's group distinct from the SDK
group when sharing a version. Actual connected tests/builds verify the graph.

The shared dispatch seam also exposes a concrete repeated allocation: CoreWallet's
derived Clone copies every immutable recovery-report ID vector per async call.
Keep the core behind Arc in the FFI wrapper so dispatch clones one shared handle;
remove the now-unused core Clone derive. Exported ownership, Send/Sync and Tokio
behavior stay the same. Existing real-wallet/in-flight-release/RegTest contracts
exercise the boundary; do not claim a measured throughput gain.

Final runtime validation reproduced a reusable-stack startup bug: the miner and
server already have sufficient funds, so bootstrap mines nothing and LND rejects
the several-hours-old best header as unsynced for the full 120-second timeout.
Before its LND readiness wait, mine one fresh block with the existing miner.
This only advances the isolated RegTest chain; never reset a volume or seed.
Validate by rerunning the same failed bootstrap and full paired acceptance.

The rebuilt debug iOS SDK reproduced a stack-guard crash twice during Wallet.sync,
inside Tokio spawn through the new async dispatch helper. First remove only the
helper's redundant async state: return run_async's future directly, preserving
lazy Tokio submission, cancellation and exported signatures. Rebuild the debug
SDK and rerun the same open/sync command before the complete suite. Do not change
thread stack sizes, generated bridge code or optimized release settings to hide it.

The post-exit fresh restore legitimately reconstructs a bark.exit/start movement
while syncing detected onchain exits. Read-only inspection confirmed a newly
created SQLite database and this one new movement. Check empty history only on
the first funded restore before exits; retain unique database IDs and complete
recovery reports on every restore. Rerun the full suite with a new test account.
