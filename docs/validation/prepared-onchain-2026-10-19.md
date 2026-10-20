# Prepared onchain sends: execution record

Engineering plan: [Noah items 14–16](../plans/20-noah-release-priorities.md).

Implemented signed, exact-fee send/drain previews, shared BDK reservations,
confirmed-only filtering, exact-byte broadcast/discard, typed BarkError codes and
accepted/failed/notAttempted results. Pending Lightning listings already expose
hash, amount and fee; migration guidance documents the amount check before resume.

Validation to date:

- Rust check and format pass. Three BDK/real-SQLite tests pass: construction
  failures and preparation leave all database/sidecar bytes unchanged; fees use
  signed many-input transactions; recorded bytes reopen exactly with no persisted
  transient locks.
- Full live Rust library suite: 88 passed, zero failures, including cancellation
  cleanup, existing schema-42 opening, balances and existing board/exit flows.
- Three additional live RegTest tests passed in 17.66s. Bitcoin Core observed
  transaction a5c1d9ccf9884f79bb0801ccead9e9a2f40de5d50ac98738786ee996bab802be:
  12 inputs, 105,000 recipient sats, 7,775 fee sats, 775 signed vB. Verified input
  identities/txid/fee/vsize, another handle, a running daemon, confirmed-only drain,
  unconfirmed inclusion, invalid rates including conversion-valid overflow, dust,
  unknown IDs, discard and a real SQLite trigger failure before submission.
- Failed-network retry regression passed through a transparent Bitcoin RPC proxy:
  an evicted old preparation cannot spend a newer reservation; retry after discard
  re-records it with a timestamp newer than eviction.
- Delayed real Ark cosigning regression passed with a separate Ark database:
  preparation waits for the whole board call and excludes its funding inputs.
  Supplied board PSBTs spending reservations are rejected before upstream execution.
- GPT-6 Astra high review found four real issues (boarding race, evicted retry,
  cancelled-result leak, fee multiplication overflow). All were verified/fixed;
  its second review approved with no remaining source-level blockers.

[Source CI 39352149601](https://github.com/smolcars/bark-bolt-ffi/actions/runs/39352149601) passed all three jobs at native source `9c895da711019e570883e4ae87bb7924c2c1fe1d`. It built all five Apple slices and three Android ABIs, passed native archive/deployment checks, Swift/iOS app contracts, strict API parity (153 exports, 52 records, 18 enums, two callbacks), live Rust (88 library + 3 integration tests), regular/packaged live Kotlin contracts (11 each), Android emulator and app acceptance. Hosted Rust live times were 31.85s + 11.81s; Kotlin reports 11 tests, zero failures/skips.

The successful hosted artifacts were downloaded and verified: Apple 12234905965 SHA256 `247d0e0906ce26a35fe29eb48458a6d684cce89a9852ba8c9bd763fbd4e97c7f`; Android 12234686494 SHA256 `8ea9c7a05bc7f69fe2d84469c818f7ffe86b915373ac08a72c929b82d88c126d`. The retained [staging recipe](v040-stage-ci.py) checks exact native-source equivalence and successful native/Swift/iOS CI steps, plus all Apple architectures and deployment minima. Staging overlapped cache saving; the full source CI subsequently completed successfully before publication.

The local live Swift consumer of that real CI-built SDK passed all 12 tests, zero failures/skips, in 40.926s, including exact-fee prepare/drain, typed invalid rate, discard/reprepare and accepted exact-byte broadcast/history. Ten Python tooling tests and Swift source provenance checks pass. The downloaded three-ABI AAR has four LOAD segments per library, all minimum Align `0x4000`.

The v0.4.0 XCFramework ZIP is 190,302,801 bytes, SHA256 `210b83c4b1f3db9f6cc7e1c59adada7531c8b9dd0a4ca15811479fca696d36fd`. PR #3 is merged. Publication, fresh downloads and public Git-URL consumer acceptance are recorded in the release follow-up.

The release carries mobile SDKs and the hosted Linux JVM JAR; Darwin JVM can be built from source. IDs are process-local; discard
cannot undo a transaction already recorded for broadcast/retry. Unknown upstream
error categories remain Other with their diagnostic chain. Physical devices and
external payment providers are outside this change's validation.
