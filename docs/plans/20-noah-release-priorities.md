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

## Items 15/16

After item 14's ownership design is settled, expose typed construction errors,
explicit broadcast outcomes and pending Lightning send hash/amount/fee details
from upstream checkpoints. Validate amount identity before resume. Record limits
and actual test outcomes here before checking off acceptance.
