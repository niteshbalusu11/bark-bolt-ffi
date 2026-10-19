# Just/Bash tooling and current Actions

Status: implementation and local runtime checks pass; updated Actions runs pending.

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
- All eight existing release/runner regression tests pass. Committed Swift
  release integrity passes; no published artifact was restaged.
- Real just setup-everything passed twice on the existing default stack. The
  reference mnemonic's SHA256 is identical before/after; balance remains
  998900 sats and the existing Lightning channel is reused.
- A new bark-bolt-ffi-just-check-20261019 project on ports19643/3835 bootstrapped
  from empty volumes, funded the miner/server/reference wallet, opened an active
  LND-to-CLN channel and reached reference spendable balance998900 sats.
- just test-regtest --extended, run just-bash-20261019-r1, passed against that fresh
  stack on the installed current SDK apps: iOS27 arm64 and Android35 arm64.
  Both apps completed Ark send/receive, BOLT11 send/receive, refresh, independent
  funded Ark/onchain restores, offboard, positive/component-checked exit fees,
  unilateral exit/claim with confirmed transactions, cancellation/reuse, invalid
  input and unavailable-server retry.
- Ordinary just down preserved all six fresh-project volumes and the reference
  seed hash. Explicit just down --volumes then removed only that new test
  project's containers and volumes. The default stack and original Nitro data
  remain intact.

## Actions validation

All six used actions were verified against upstream releases/latest and resolved
release-tag commits on 2026-10-19. Full commit pins include exact version comments.
Their Node24 runtimes are compatible with the existing latest-stable runner
v2.338.0. JDK17 and the existing SDK/NDK/platform compatibility targets are retained.

Hosted SDK/app and actual paired runner workflows will be recorded here after
completion. Shell checks also run in the hosted Rust job.

## Scope

SDK wallet logic, generated public bindings and native app behavior did not
change. No v0.1.0 release asset/tag or committed generated snapshot was replaced.
Physical/minimum-OS runtimes, real device transfers and external Lightning
provider fixtures remain outside runtime coverage.
