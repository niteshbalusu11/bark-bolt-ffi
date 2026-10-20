# Noah priority items 13–16

These are Noah's item numbers, distinct from the original migration task numbers.
Execute Android item 13 and its patch release before changing the wallet API.

## Item 13: Android 16 KB ELF alignment, v0.3.1

Affected paths: `.cargo/config.toml`, Android artifact validation scripts and
Python tooling tests, `.github/workflows/ci.yml`, Cargo version/lock, release
metadata and this execution checklist. Keep all three shipped Android ABIs.
Set `-C link-arg=-Wl,-z,max-page-size=16384` for aarch64-linux-android,
x86_64-linux-android and armv7-linux-androideabi in target-specific Cargo config
for direct Cargo builds. BoltFFI relinks static archives into the final JNI `.so`
with its own clang command, so also set `targets.android.link.extra_args` in
`boltffi.toml` to the same flag. Repack after this configuration change and
validate the final AAR. No wallet code,
persisted data or consumer API changes belong in this patch.

Validate the assembled AAR, extracting every `.so` (including any future
additional native library), using the NDK's `llvm-readelf -lW`. Reject missing
LOAD entries, malformed output and any LOAD Align below 0x4000. Tool resolution
must work on the local Mac and Linux CI; a missing tool fails the gate.
Run parser regression cases, reproduce rejection on the published v0.3.0 AAR,
build the three real optimized Android libraries/AAR and check all LOAD entries.
Build/stage the versioned Apple/JVM assets, run packaged consumer contracts,
record hosted CI separately, then publish immutable v0.3.1 assets/checksums and
verify fresh downloads. Never substitute a parser test for an ELF build result.

## Item 14: prepared onchain sends

Before implementation, refine this section against pinned Bark/BDK interfaces:
affected onchain core/FFI, DTOs, shared-wallet adapters, tests and SDK callers;
ownership of signed transactions/reservations; locking across daemon sync,
boarding, exits and legacy sends; construction with no persistence; exact signed
fee/vsize/input reporting; confirmed-only filtering; broadcast persistence/retry
and discard lifetime. Preserve upstream persistence and validation. Check real
many-input and drain transactions against RegTest and both generated SDKs.

### Refined implementation decisions

- Keep the release build checkout frozen; implement the API on the
  `feature/prepared-onchain` worktree, merging only after v0.3.1 publication.
- `src/core/onchain.rs` owns signed transactions and reservations together under
  the same Tokio write lock as BDK. `src/ffi/onchain.rs` exposes prepare/broadcast/
  discard; `src/types.rs` holds `OnchainSendAmount` (Amount/DrainAll),
  `PreparedTransaction` and `BroadcastResult`/`BroadcastOutcome`. Reuse `OutPoint`.
- Use BDK 3.1's native outpoint locks so upstream ordinary sends, drains,
  boarding and CPFP coin selection all exclude prepared inputs. Remove only
  transient reservation entries from BDK's staged lock changes immediately, so
  sync cannot persist abandoned locks or a prepared transaction. Keep its signed
  bytes outside BDK's graph until broadcast. The daemon uses the same state.
- Cache live onchain state by canonical SQLite path, with network/seed identity
  checks, so another SDK handle cannot bypass reservations. Retain this state
  and the database registration through the daemon's onchain trait handle.
- Reject zero/overflow rates, zero/dust amounts, invalid/network-mismatched
  addresses and BDK construction failures as typed errors. `confirmed_only`
  uses BDK `exclude_unconfirmed`, including for drain. Report PSBT fee and final
  signed transaction vsize, recipient value and exact previous outpoints.
- Broadcast registers/persists the exact signed bytes first, then submits those
  bytes, retaining upstream retry-on-sync behavior. Persistence failure reports
  notAttempted and keeps the reservation; network failure reports failed(reason)
  with txid and allows retry of the same prepared id. Discard before broadcast
  unlocks; after recording it only forgets the handle and cannot cancel a spend.
- Prepared ids and reservations are process-local; dropping all handles releases
  unbroadcast preparations. They are never restored from wallet backups.
- Extend existing tests rather than add dependencies: many small confirmed
  inputs, unconfirmed filtering, drain, prepare/error/discard without DB changes,
  sync and competing send/board/CPFP protection, shared handles, unknown ids,
  persistence/network failure and exact node-observed tx bytes/fee/vsize. Compile
  both generated host languages and exercise real RegTest via the existing
  isolated project. Preserve all unrelated Noah/Nitro containers and volumes.

### Astra review repairs (confirmed before publication)

Serialize prepare/broadcast with whole board_amount/board_all/board_psbt calls
using a shared operation gate. Read durable board funding inputs from the onchain
persister and every attached Ark persister before selection; attach under this
gate before daemon startup and reject overlaps with active preparations. Validate
external board PSBTs against reservations before calling upstream.
Every broadcast retry must recheck current locks/canonical inputs and re-record
an evicted own transaction with last_seen newer than last_evicted. Keep foreign
broadcasts detached; use an RAII handoff for preparation so cancellation discards
an unclaimed id. Check fee multiplication for maximum Bitcoin transaction weight,
including conversion-valid overflowing rates. Add race/cancellation/eviction/
overflow regressions, then ask Astra to verify all four repairs.

## Items 15/16

After item 14's ownership design is settled, expose typed construction errors,
explicit broadcast outcomes and pending Lightning send hash/amount/fee details
from upstream checkpoints. Validate amount identity before resume. Record limits
and actual test outcomes here before checking off acceptance.
