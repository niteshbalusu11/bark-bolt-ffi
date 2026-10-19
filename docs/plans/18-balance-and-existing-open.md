# v0.3.0: complete balance and existing-wallet open

## Engineering plan (before implementation)

- Paths: `src/types.rs`, `src/ffi/{db,onchain,wallet}.rs`, Rust tests,
  `tests/{swift,kotlin,fixtures,support}`, `docs/api-baseline.json`, migration/checklist
  documentation, serialized live Rust CI, Cargo version/lock, generated release Swift SDK and SPM manifest.
- Balance: preserve all six existing fields, append the three missing buckets
  and Bark-computed pending/total. Keep Bark's classification, ownership,
  persistence and transaction validation unchanged. Do not duplicate sums in FFI.
- Opening: introduce `OnchainWallet.openExisting` for callers expecting an
  initialized database; retain `default` for creation. Share read-only validation
  with `Wallet.open` when `createIfNotExists=false`. Verify initialized properties,
  network and Bark's seed fingerprint before any migrating/BDK connection.
  Accept schema 42 and compatibility filename selection. No failed preflight may
  change input bytes, tables, sidecars or directory entries.
- Ownership/concurrency: no new persistent handle or background task. Validation
  precedes existing open paths; preserve current native handle lifetimes.
- Diagnosis: the user's two root causes are confirmed in the pinned source.
  Regress empty-file mutation through the real onchain factory before fixing;
  compare conversion directly with Bark and exercise genuine wallet state.
- Dependencies: pinned Bark bfbe6b7c and BoltFFI 0.31.1, existing SQLite/seed
  helpers, isolated project RegTest services, local toolchains and GitHub access.
- Validation: Rust fmt/Clippy/tests, strict Swift/Kotlin generation/API checks,
  actually compiled/executed Swift and Kotlin contracts, funded RegTest balance
  comparisons including refresh threshold and in-flight action buckets. A transparent TCP
  proxy delays real server responses so transient locks can be observed without
  fabricating transactions or changing upstream logic. Run live Rust tests with
  `-- --test-threads=1`: expiry mining must not age another test's wallet. Rebuild
  optimized five-slice XCFramework, three-ABI AAR and host JAR, stage provenance,
  verify SHA256SUMS, publish v0.3.0 and run a clean public Swift consumer. Hosted
  CI must pass before its checklist items can be complete.

## Actual results

Implemented both interfaces and all Rust/Swift/Kotlin contracts. Final rebuilt
SDKs passed 84 Rust, 12 Swift and 11 compiled/executed Kotlin contracts, plus
11 packaged-JAR contracts. Both Fable 5.1 high review rounds completed and all
findings are repaired. Full hosted source CI passed on the exact tagged commit.
v0.3.0 is published with eleven assets; fresh downloads match SHA256SUMS and a
clean public Git consumer passed all 12 live Swift contracts. Noah received and
acknowledged the migration notice. [Execution evidence](../validation/2026-10-19-v030.md)
records commands, hashes, CI links and the separate automatic tag rerun's state.
Task 18's acceptance gates are complete.

## Compatibility refinement before final publication

The initial nonempty rollback-journal guard can reject valid SQLite PERSIST
wallets whose successful commits leave a zeroed journal. Add a regression for
that real database mode before changing `src/ffi/db.rs`. Use ordinary read-only
SQLite for rollback-mode databases, letting SQLite distinguish hot/cold journals;
use immutable mode only for a closed WAL-mode database (header bytes 18/19).
Retain the missing-WAL-index guard. Validate source/directory immutability on
rejection, rerun native contracts and rebuild/restage all release artifacts.

## Requested review and consumer handoff

Before publication, delegate review of the complete baseline-to-working-tree
change to Claude Fable 5.1 with effort high. Independently reproduce/validate its
findings, record a plan before any fixes, and rerun affected checks/rebuilds.
After publishing and verifying v0.3.0, notify Noah's active native rewrite thread
`4ff9b718-306c-496c-831f-8edfa1484899` with the release URL and migration details.
Do not announce publication to that thread before the assets are public.

## Fable review repairs (plan before implementation)

Fable 5.1 high round 1 approved the balance mapping and read-only write ordering,
with one should-fix and two suggestions. Validate each against the actual Rust
SQLite build before accepting it.

1. Hot rollback journal: create a real in-flight transaction, flush its pages,
   copy main+journal to an isolated directory, roll back the source, and invoke
   the preflight. If confirmed, distinguish missing properties from
   SQLITE_READONLY_ROLLBACK and other read errors. Preserve all bytes/files on
   rejection. Do not auto-recover or fall back to creation.
2. Empty/truncated header: add specific context and assert its text from Rust,
   Swift and Kotlin rejection contracts.
3. Concurrent zero-length WAL: inspect Noah's journaling usage; prefer read-only
   SQLite with existing WAL/index files even when the WAL has zero bytes. Only
   an absent WAL should select immutable mode. Reject missing indexes without
   creating them; test zero-length live and missing-index inputs.

Affected paths: `src/ffi/db.rs`, host rejection contracts, temporary test fixtures
if required, migration/validation documentation. No balance/transaction logic or
API shape changes. Re-run targeted Rust repros, the full native checks, strict
SDK builds, and Fable verification before release staging/publication.

## Fable round 2 outcome and final test refinement

Fable approves with no remaining blocker and verifies all three repairs closed.
Its optional remaining suggestion is to pin the uninitialized-properties error
for a foreign SQLite input. Add that assertion to both foreign-language
rejection contracts (both factories), so the Rust diagnostic is checked across
FFI while preserving the frozen native source. Run the already-planned compiled
Swift and Kotlin suites on the rebuilt artifacts. No native behavior/API changes.
