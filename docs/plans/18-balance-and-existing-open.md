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

Implemented both interfaces and all Rust/Swift/Kotlin contracts. Rust live and
Swift release contracts passed; Kotlin runtime, hosted CI and publication are
in progress. [Execution evidence](../validation/2026-10-19-v030.md) records actual
commands, artifact checks and remaining release gates. Task 18 stays open until
those gates pass.
