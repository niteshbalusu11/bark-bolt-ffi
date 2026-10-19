# Just/Bash tooling and current Actions

Status: complete. Changes are pushed; local checks and both Actions workflows pass.

[Engineering plan and exact action releases](../plans/16-just-and-current-actions.md).

## Restored workflow

The root justfile and scripts/ark-dev.sh restore Nitro Ark's command family:
setup-everything, up/down/stop, bcli, aspd, bark, lncli, cln, generate, send-to,
fund-aspd, create-wallet, create-bark-wallet and setup-lightning. The existing
isolated Compose files and current-master image digests are unchanged.
The Python backend manager is deleted. Both CI workflows use Just recipes for
SDK builds and RegTest; the paired job also cleans up with just down --volumes.

Python remains behind Just for device JSON polling/assertions, device readiness
and the ephemeral runner's archive/environment validation. It does not duplicate
backend setup/funding/mining logic. Historical validation commands describe their
original runs; active documentation uses the restored entry points.

## Local checks

- Bash syntax and ShellCheck pass. Focused shell checks exercise literal JSON,
  empty/whitespace/newline argument forwarding through Bash and Just, ordinary
  down's volume preservation, existing-wallet reuse, fail-closed wallet lookup,
  nonzero CLI exits and invalid block counts.
- The original raw captaind interface also passes: real `just aspd --version`
  reports master bfbe6b7, `just aspd rpc wallet` succeeds, and setup still passes
  after preserving that exact passthrough shape. Shell checks cover these args.
- All eight existing release/runner regression tests pass. Committed Swift
  release integrity passes; no published artifact was restaged.
- Real just setup-everything passed twice on the existing default stack. The
  reference mnemonic's SHA256 is identical before/after; balance remains
  998900 sats and the existing Lightning channel is reused.
- A new bark-bolt-ffi-just-check-20261019 project on ports 19643/3835 bootstrapped
  from empty volumes, funded the miner/server/reference wallet, opened an active
  LND-to-CLN channel and reached reference spendable balance of 998900 sats.
- just test-regtest --extended, run just-bash-20261019-r1, passed against that fresh
  stack on the installed current SDK apps: iOS 27 arm64 and Android API 35 arm64.
  Both apps completed Ark send/receive, BOLT11 send/receive, refresh, independent
  funded Ark/onchain restores, offboard, positive/component-checked exit fees,
  unilateral exit/claim with confirmed transactions, cancellation/reuse, invalid
  input and unavailable-server retry.
- Ordinary just down preserved all six fresh-project volumes and the reference
  seed hash. Explicit just down --volumes then removed only that new test
  project's containers and volumes. The default stack and original Nitro data
  remain intact.

[Local acceptance and preservation evidence](just-bash-20261019-r1.json) records
the original 279031a run. The public aspd passthrough correction was separately
verified locally and in the final f66a75d paired Actions run below.

## Actions validation

All six used actions were verified against upstream releases/latest and resolved
release-tag commits on 2026-10-19. Full commit pins include exact version comments.
Their Node 24 runtimes are compatible with the existing latest-stable runner
v2.338.0. JDK 17 and the existing SDK/NDK/platform compatibility targets are retained.

[Paired Actions run 39260224145](https://github.com/smolcars/bark-bolt-ffi/actions/runs/39260224145)
passed at f66a75d510bd79738e6cb68cfe0e30dfcef90a5c, using checkout 7.0.1 and
upload-artifact 7.0.2. All 64 device commands passed, including two distinct fresh
restores per app. The latest artifact upload downloaded successfully. The runner
unregistered and removed its temporary directory, and just down --volumes left
no containers/volumes for that exact CI project. [Durable evidence](integration-39260224145.json).

[Hosted SDK/app run 39260131486](https://github.com/smolcars/bark-bolt-ffi/actions/runs/39260131486)
passed all three jobs at the same final implementation commit:

- Rust: eight existing tooling regressions, ShellCheck and shell safety tests,
  formatting, warnings-denied Clippy, 57 Rust tests and audit.
- Android: all three release ABIs, strict generation/API parity, AAR/JAR checks,
  ordinary and actual-JAR contracts including real RegTest, minified release/debug
  app builds, two emulator seed/UI tests and app RegTest smoke. Fresh backend
  setup and app testing run through Just/Bash on hosted Linux.
- Apple: all five release slices and deployment-target checks, three Swift
  contracts, three app unit tests and one actual iOS UI test. SDK builds run
  through the Just recipe; test/SDK artifacts upload successfully.

The initial 279031a hosted run was superseded by the passthrough correction;
final acceptance uses the successful f66a75d workflows above. The SDK, app source
and release-build script are unchanged from 95d90fd.

## Scope

SDK wallet logic, generated public bindings and native app behavior did not
change. No v0.1.0 release asset/tag or committed generated snapshot was replaced.
Physical/minimum-OS runtimes, real device transfers and external Lightning
provider fixtures remain outside runtime coverage.
