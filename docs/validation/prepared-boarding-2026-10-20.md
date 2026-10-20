# Prepared boarding, chain status and stored exits: v0.5.0

Engineering plan and upstream feasibility: [Noah follow-up](../plans/23-noah-prepared-boarding.md).

Implemented: exact signed board amount/drain previews with gross/net/Ark/onchain
fees, signed vsize and selected outpoints; process-local shared reservations;
exact-byte submission/retry/discard; typed construction/input/expiry errors;
accepted/failed/notAttempted results. Direct chain transaction status is exposed
on Wallet and BDK OnchainWallet. listAllExits reads persisted live/terminal rows
with their exact VTXO amounts and optional history/transactions. No Bark database
schema, transaction-validation or fee algorithm was replaced.

GPT-6 Astra high completed a read-only background source review. It found a real
pre-sync reservation gap after boarding and a spend-check gap after a reorg below
the cached scan height. The implementation now commits the exact funding tx to
BDK before releasing its locks, protects pending-board inputs in every legacy
builder with Drop-safe temporary locks, and validates confirmed input availability
independently of the recent-spend scan. The reviewer inspected those fixes and
reported no further concrete money-loss or deadlock defect. Regression outcomes
and final release evidence are recorded below as they complete.

Initial local validation before the review refinements: check/format/all-target
Clippy passed; the full live Rust suite passed 88 library + 5 integration tests.
A real 12-input board matched Bitcoin Core exactly: txid
`915f467edadcc7ff2ca2c0d67fb5efd4387dd978cf8d683681e288a5dcc7dcbc`,
112,199 net sats, 330 Ark fee sats, 7,471 onchain fee sats, 744 signed vbytes.
Preparation and construction failures left every SQLite/sidecar byte unchanged.
Both canceled and actually claimed exits survived reopen with history/packages.
Ten Python tooling tests passed.

The first reorg regression was mistakenly overlapped with another wallet test on
the shared test node and hit a Bitcoin Core block-index assertion. The original
volumes were preserved; the node was reindexed/recovered and the Ark server
replayed its chain. The committed reorg test owns a separate ephemeral Bitcoin
container and cannot rewind the Ark/Lightning stack. Nitro/Noah's `scripts-*`
containers and volumes were not changed. No production funds were involved.

The local Apple build initially failed loading existing release proc macros
(`mis-aligned LINKEDIT string pool`) on macOS 27 / Xcode 27. Rebuilding host build
dependencies with `CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_DEBUG=1` and
`CARGO_PROFILE_RELEASE_STRIP=none` passed that loader failure. This is a local
build workaround; published SDKs must come from the full main CI matrix.

Remaining product scope: prepared offboards / Ark sendOnchain and atomic durable
operation keys are not implemented. See the exact upstream seams in the plan;
no amount matching or wrapper estimates are presented as their replacement.

Post-review validation: all three prepared-board live tests passed serially in
128.25s, including real pre-movement and post-acceptance funding-cache SQL trigger
failures, same-ID retry/discard, legacy send/drain/board protection after explicit
local eviction, external mempool-spend rejection, actual exit claiming/reopen,
and an isolated-node below-tip reorg. The first test time includes the restored
Ark node's chain replay. All-target Clippy with `-D warnings` passes after the
fixes. Astra's final review passes the implemented scope with no remaining
concrete blocker.

Known direct coverage gaps: board preparation-result cancellation, expiry
rejection, simultaneous board/exit-CPFP calls, and a failed-broadcast pending board after
reopening. The shared reservation helpers,
existing prepared-send cancellation/daemon contracts and real exit CPFP run
cover related paths; they are not claimed as direct tests of these cases.

Final local Rust run after the code fixes: 88 library tests passed in 75.53s,
3 prepared-board live tests in 54.63s, and 3 existing prepared-send live tests in
21.10s, zero failures. A further focused run passed all 3 board tests in 32.90s
with two added real regressions: an active board reservation blocked exit CPFP
until discard, and 101 inputs were rejected without SQLite writes or leaked
reservations. Strict Kotlin generation passed with the new APIs. Fresh iOS device,
iOS simulator and macOS ARM64 release libraries compiled locally; binding/package
validation and the full hosted architecture matrix remain pending.
