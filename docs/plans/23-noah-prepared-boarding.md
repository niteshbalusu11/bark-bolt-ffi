# Noah follow-up: prepared boarding and durable exit listings (v0.5.0)

## Feasibility at Bark bfbe6b7

1. Prepared boarding is supported using `peek_next_keypair`,
   `board_funding_address`, BDK's signed draft builder, and `board_psbt`.
   Construction must not store a key, movement, checkpoint, transaction, or
   reservation. Transient shared BDK locks are process-local. Submission stores
   the deterministic key and passes the exact signed PSBT to Bark, preserving its
   fee validation, server cosign verification, checkpoint and recovery logic.
2. `ChainSource::tx_status` is public and read-only. Expose it on Wallet and the
   BDK OnchainWallet; propagate transport failures rather than reporting notFound.
3. `Exit::list_all` is public and reads persisted live and terminal rows. Expose
   its optional history and transactions and load the amount from the exact VTXO.
4. Prepared offboard/sendOnchain cannot meet the requested contract through the
   current high-level APIs. `start_offboard`, input selection and access to the wallet's server connection
   for fee-rate negotiation are private; preparing SendOnchain derives/stores keys and its
   executor may perform an arkoor split and sign forfeits. Public action structs
   do not provide shared process-local VTXO reservations across every spender and
   the daemon. A wrapper estimate plus normal submit would silently reselect and
   reprice. Keep these requests open pending upstream prepare/commit entry points
   and a reservation-aware selection hook. The lower-level server-rpc
   `offboard_feerate` method and public `Offboard`/`drive_action` types exist;
   assembling them in the FFI would duplicate wallet orchestration without
   supplying the missing shared reservation and preparation guarantees. Server acceptance of a caller-selected
   rate also needs a protocol decision; it cannot be guaranteed client-side.
   Baseline correction: this pinned Bark already checkpoints offboards and commits
   their fees when starting. Its `run_offboard` returns a txid only from an
   AwaitingConfirmations checkpoint; an action parked before broadcast returns an
   error and remains pending. This improves the older bare-txid behavior, but
   does not supply the requested preview/submit API or explicit outcome DTO.
5. Generic durable operation keys need an upstream persistence hook tying a
   caller key to the initial checkpoint/movement before any network side effect.
   Delegated refresh, exit progression and claim do not expose this. An extension
   journal surrounding the existing calls leaves a crash gap with an unknown
   outcome; never report failedWithoutBroadcast from that gap or match amounts.
   Keep this request open. Boarding's deterministic funding txid alone does not
   implement the requested cross-operation idempotency contract.

## Engineering plan before implementation

Affected paths: src/core/prepared_onchain.rs, src/core/prepared_board.rs,
src/core/{mod,wallet,onchain}.rs, src/ffi/{wallet,onchain}.rs, src/{types,error}.rs,
Rust/native contracts, docs/api-baseline.json, README and migration guidance,
Cargo manifests, Gradle version and release metadata.

Ownership/interface: Wallet.prepareBoard uses OnchainSendAmount (amount or
DrainAll) and returns PreparedBoard with exact gross/net/Ark/onchain fees,
signed vsize and funding outpoints. Wallet.submitPreparedBoard returns BoardResult
with BroadcastOutcome and VTXO IDs; discardPreparedBoard releases only an
unsubmitted reservation. Store the signed PSBT, derived key/index, expiry, fee
snapshot and Ark database owner in the existing shared BDK WalletState. Reject
callback-only wallets with UnsupportedOperation. Restrict send/broadcast/discard
methods to their preparation kind. An undelivered FFI preparation result releases
its inputs. IDs remain process-local, and submitted boards remain durable Bark
actions after discard/reopen.

Invariants: no preparation database writes or broadcasts; exact submitted bytes;
no overlap with sends/drains/boards/exit CPFP; revalidate spendability, server board
parameters and expiry before submission; typed dust/minimum/fee/input errors;
never rebuild after review. Hold the shared operation gate through board cosign
and durable checkpoint creation, retaining transient locks until that checkpoint
protects inputs. Bark's initial best-effort broadcast hides its result, so make an
explicit exact-byte broadcast after acceptance and return that attempt's outcome.
Do not classify a failed transport as proof that the network never received it.
Retry a submitted process-local ID by broadcasting its existing transaction,
without creating another movement or board. Exclude pending board inputs from
all draft construction and prevent generic APIs from submitting a board as a send.

Validation: format/check/Clippy; real-SQLite no-write preparation and failure
checks; live RegTest exact fees/txid, discard/retry, shared handles, concurrent
reservations, fee/expiry invalidation, typed errors, cancellation handoff; chain
notFound/mempool/confirmed without wallet sync; cancel/reopen and claimed/reopen
exit listings with history and transaction flags. Generate both SDKs strictly,
compile Swift/Kotlin contracts and build fresh native artifacts. Run hosted CI
and record separately from local results. Request GPT-6 Astra high background
review after implementation, fix confirmed issues and rerun affected checks.

Release and cleanup: bump to v0.5.0 only for the implemented scope. Publish only
artifacts built from the reviewed revision with checksums and provenance. Keep
unimplemented requests explicitly open in the execution checklist. Save test
outcomes before removing disposable builds and task-created devices. Preserve
pre-existing Nitro/Noah Docker volumes, shared runtimes and user devices.

## Review-driven refinement before the next changes

Astra identified that Bark's best-effort funding registration is refused while
our transient locks are held. Releasing them immediately after acceptance leaves
legacy BDK send/drain/board/CPFP paths able to select the funding again before
sync. Fix in `WalletState`: commit the exact funding tx before release, retaining
locks if that write fails, and consult durable pending-board inputs in every
legacy builder using a Drop-safe temporary lock guard. Include core send/drain
(which otherwise dereference directly to upstream BDK) and foreign board PSBT
validation. Regression: legacy send/drain/board must fail immediately after
submit without sync; also exercise rejected broadcasts and persistence failures.

Independent chain checks must catch reorg-conflicting spends below the numeric
scan start. Check parent status and confirmed outpoint availability as well as
the recent-spend scan; an already-visible own funding transaction is exempt from
checking its own consumed inputs. Test a real invalidateblock/re-mine scenario.

No-write prerequisite: a wallet created offline with no pinned server keys must
sync once before preparation. Reject it without writes rather than let upstream
lazy connection store identity keys. Construct funding script/expiry directly
from the checked ArkInfo snapshot and BoardBuilder. Pinned server-rpc initializes
its ArkInfo cache once; heartbeat does not mutate fee parameters on this revision.

## v0.5.0 cleanup ownership refinement

After public asset and tagged-consumer verification, remove this checkout's
ignored `target`, `dist`, `artifacts`, `tests/swift/.build` and empty `.kotlin`
outputs, plus task-specific temporary artifact archives/extractions. Check that
no tracked file is under a removal path and preserve committed SDKs/evidence.
No simulator or emulator was created/opened by this task; the booted Noah devices
and shared runtimes stay in place. The reorg fixture's disposable Bitcoin
container is already removed by its guard. Preserve the pre-existing repository
RegTest stack and every Nitro/Noah volume. Measure allocated output bytes before
removal and available filesystem bytes before/after; these differ when concurrent
threads use the same APFS volume.
