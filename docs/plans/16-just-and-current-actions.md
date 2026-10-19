# Restore Just/Bash RegTest commands and update Actions

User preference: use the original Nitro Ark Justfile/ark-dev.sh workflow rather
than a replacement Python stack manager. Keep Swift/Kotlin SDK/app behavior and
published v0.1.0 artifacts unchanged.

## Interfaces and implementation

- Add the root justfile and scripts/ark-dev.sh, adapted from Nitro Ark's MIT
  scripts. Retain setup-everything, up/down/stop, create-wallet,
  create-bark-wallet, generate, fund-aspd, send-to, aspd, bark, bcli, lncli, cln
  and setup-lightning commands. Use the existing isolated regtest/compose.yaml
  and environment-selected CI project/ports. Remove scripts/regtest.py.
- Preserve the current miner/reference wallet locations, bounded service/channel
  readiness, mature funding, idempotent wallet/channel setup and fresh chain tip.
  Retain seeds/databases and ordinary down's volumes; do not copy Nitro's forced
  Bark reset or automatic down --volumes. CI cleanup still explicitly removes
  only its own per-run Compose project.
- Python remains for mobile command/JSON polling/assertions and the ephemeral
  runner's environment/archive validation. App automation invokes ark-dev.sh for
  every backend operation, with no separate Python stack implementation.
  Expose both helpers through Just recipes so the daily workflow is Just/Bash.
- Update active README/device/CI instructions and original plans that reference
  the removed backend manager. Historical validation commands stay historical.
- Update every third-party uses entry to the latest stable GitHub release,
  retaining full commit pins and exact version comments. Verify each release
  through its upstream GitHub release/tag API. Keep JDK17, SDK/NDK and platform
  targets pinned; this request updates actions, not the SDK compatibility floor.

## Releases verified on 2026-10-19

| Action | Latest stable | Commit |
| --- | --- | --- |
| actions/checkout | [v7.0.1](https://github.com/actions/checkout/releases/tag/v7.0.1) | 3d3c42e5aac5ba805825da76410c181273ba90b1 |
| actions/cache | [v6.1.0](https://github.com/actions/cache/releases/tag/v6.1.0) | 55cc8345863c7cc4c66a329aec7e433d2d1c52a9 |
| actions/setup-java | [v6.1.0](https://github.com/actions/setup-java/releases/tag/v6.1.0) | b843ebf9b913146b38da2eaa7aeddf76074ae1ad |
| actions/upload-artifact | [v7.0.2](https://github.com/actions/upload-artifact/releases/tag/v7.0.2) | cf430e030ddbb5b0abf93d22962f4752f3646cd9 |
| android-actions/setup-android | [v4.0.4](https://github.com/android-actions/setup-android/releases/tag/v4.0.4) | be39fa834029ff78f1a44aa3bb0819b8fc2bd8fd |
| ReactiveCircus/android-emulator-runner | [v2.38.0](https://github.com/ReactiveCircus/android-emulator-runner/releases/tag/v2.38.0) | a421e43855164a8197daf9d8d40fe71c6996bb0d |

The pinned ephemeral runner v2.338.0 is already the latest stable release and
matches its upstream asset SHA256. Check action metadata/runner compatibility
before pushing. All selected actions use Node24; the minimum documented runner
is 2.327.1 (2.329.0 for authenticated container checkout), below our 2.338.0.

## Validation and completion

Before runtime changes: capture existing wallet identity/state for preservation.
Check Bash syntax/ShellCheck, Just argument forwarding/listing, existing tooling
regressions, and focused shell safety tests for no forced reset, failure
propagation and quoted CLI arguments. Run real setup twice on the existing
stack, then test a fresh isolated local project and a fresh CI project via the
normal workflow. Build/install the
existing SDK apps as needed and execute the full paired RegTest suite through
Just. Verify hosted SDK/app CI with all updated actions and an actual manual
paired run through the runner helper, including exact-project cleanup. Push
implementation and record real results/remaining limits before completing task16.
