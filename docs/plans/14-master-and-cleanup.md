# Current-master alignment and final cleanup

User steering: remove unnecessary code, align with current Bark/Nitro master,
reuse Nitro's RegTest approach, and use the newly published dated images.

Evidence: Nitro master988b4f54 (2026-10-08) still pins Bark8661f24e and Docker
nightly2026-09-04. Bark master is bfbe6b7c3088a09dd4e81c4ac24cdc763a627a8c
(2026-10-16). User's bark:nightly-2026-10-19 reports that exact commit, with
amd64/arm64 manifest digest6ddad8bd07a21096d8487b8caa1825185f43b9d8ea1a2f2644bd82206a7122ba.
captaind:nightly-2026-10-18 currently reports0.7.1+8f5d8c4; confirm any newer
server tag while updating independent code. Do not infer a dated tag's source
revision from its name. The CLN image remains current and multi-architecture.

Paths: Cargo.toml/lock, Rust adaptation if master APIs changed, regtest/compose.yaml,
captaind config and scripts, release metadata, docs and hosted workflows.
Pin all Bark workspace crates to the same explicit master SHA, keep compatible
security patches and all128 original exports. Regenerate rather than patch host
code. Keep persistent filenames, ownership, transaction validation and isolated
volumes. Use user's multi-platform Bark/server images, not forced amd64 upstream
images. Inspect current upstream default server config before applying old settings.
Bootstrap a fresh isolated project for updated backend; preserve existing runs.

Delete experiments/boltffi now superseded by production contracts; delete unused
header image and old feasibility assessment. Keep required build/test scripts,
API baseline, engineering plans and actual validation evidence. Fix any links.
Check direct Cargo dependencies against actual use; retain feature-unification
requirements such as bundled SQLite. No speculative refactor of wallet logic.

Validation: cargo fmt/check/Clippy/test/audit; strict Swift/Kotlin generation and
parity; full release ABI matrix; packaged consumer contracts; new app builds and
extended RegTest on both devices; native UI checks where touched. Replace stale
Swift release archive/provenance before publishing. Prior0.7.1 acceptance remains
historical evidence, not proof for master. Complete real hosted CI again.

User confirmed the latest available captaind tag. Use nightly2026-10-18 plus its
multiarch digest9ef847236b92564d694b0ab1ed9a0c5d5dd1baa428840411376010610cb4cb8e.
Its image entrypoint already creates/starts the server from CAPTAIND_CONFIG_PATH;
reuse that script instead of duplicating the shell command. Set the config path
to the mounted file, preserve idempotent mnemonic detection. CLI bypasses its
interactive run.sh with entrypoint bark so explicit datadirs remain isolated.

Master adaptation findings: upstream replaces integer heights/deltas with typed
BlockHeight/BlockDelta, adds terminal VtxoSwept state, changes pending-exit balance
to an infallible Amount, and replaces stateful CPFP fee-walk estimation with pure
arithmetic plus optional fee_margin. Keep FFI heights as u32 with explicit
conversions, reject >65535 block deltas/confirmation counts, expose the swept
state/outpoints, remove obsolete `fundable` rather than inventing a value, and
forward optional fee_margin (defaultNone). Pending exit total comes from the
current upstream wallet balance and propagates errors.

OnchainWalletTrait adds sign_psbt (sign without storing). Add an eleventh foreign
callback with that exact semantic, retaining finish_psbt as sign+store; never
silently map one to the other. Share only decode/unsigned-transaction validation.
Remove the deleted CPFP-walk stub. Update host callback contracts and meaningful
Rust regressions for overflow, immutable signing envelopes and swept-state data.

Direct dependency scan: `thiserror` has no remaining use after the BoltFFI error
conversion. `rusqlite` is present only to enable bundled SQLite; current master
provides `sqlite-bundled` for that exact purpose. Use that upstream feature and
remove both unnecessary direct dependencies. The obsolete per-language compile
environment user-agent helper has no setter in this unified SDK; remove it and
retain the explicit Config.user_agent field. No seed/persistence behavior changes.

Remove the machine-specific simulator UUID from the acceptance driver's default;
require --ios-device or IOS_DEVICE for iOS runs. Existing documented commands and
Actions already supply it; Android-only hosted runs remain independent of iOS.

Recovery regression: integration39198410963 restored0 while the source wallet
held1990572 sats. A minimal upstream-CLI probe against the same server recovers0
from a funded998900-sat wallet in seconds, so the symptom is below BoltFFI/apps.
Server logs show RecoverVtxosByXpub, GetVtxosStatusByIds and GetVtxosByIds return
Unimplemented on the older0.7.1 server. During execution, user captaind tag
nightly2026-10-19 appeared (04:02UTC); its binary reports exactmasterbfbe6b7c.
Use that latest available tag/digest42b7bb01c7e7e9872f730a31c3132f5b0678dacca11b483d7309c64c86c5f649,
within the user's confirmed latest-tag direction. Upgrade only this project's
server and retain its funded data; rerun the identical CLI probe before full
app acceptance. No fallback RPC shim or altered wallet logic is justified.

External Maven consumer testing exposed transitive Compose x86 natives making
the APK advertise unsupported x86. Limit the example app's ABI filters to the
SDK's arm64-v8a/armeabi-v7a/x86_64 matrix, document the consumer requirement and
rebuild the external minified APK. This changes app packaging only.

Final documentation cleanup: retain Nitro's MIT notice for adapted RegTest
infrastructure under regtest/LICENSE, link it from README, and preserve the
original root license. Consolidate stale progress notes into actual task
completion records; retain historical validation separately. Verify Markdown
relative links, generated provenance, git whitespace and clean remote agreement.
Documentation-only completion records do not require another native rebuild.
