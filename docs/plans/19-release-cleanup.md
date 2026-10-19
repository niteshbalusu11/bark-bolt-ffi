# Post-release disk cleanup

The user explicitly requested closing and deleting the emulator and Docker data
used for this work after the verified v0.3.0 release.

## Plan before removal

- Ownership: remove Docker projects whose Compose configuration points into this
  repository (`bark-bolt-ffi`, `bark-bolt-master`, `bark-bolt-ffi-fresh` and old
  `bark-bolt-ffi-ci-*` fixtures), including their test volumes and networks.
  Preserve the unrelated `scripts` project used by Noah/Nitro and shared images.
- Devices: close/delete the task-created `BarkRegtest` Android AVD. Close the
  iPhone 18 Pro used for Bark acceptance and remove Bark's app/test data. Preserve
  all Noah-named devices and shared Android/iOS runtimes. Only erase the generic
  iPhone if inventory shows its non-system apps belong solely to this task.
- Disk paths: remove this checkout's ignored `target`, `dist`, `build`, SwiftPM
  test build and Android/JVM/example build outputs. Verify no tracked files are
  inside a removal path. Preserve committed generated SDKs, source, fixtures,
  attribution, release checksums/evidence and Git history. Published binaries
  have already been downloaded and checksum verified independently.
- Validation: inventory names, ownership and disk usage before removal; retain
  machine-readable release evidence first; then verify owned containers/volumes
  and AVD are absent, the Bark iPhone is shut down, unrelated environments remain,
  Git contains only intended documentation changes, and measure recovered space.
- Dependencies: completed public consumer and local live acceptance. Remote tag
  CI is independent of the local test environment and can finish after cleanup.

## Actual outcome

Removed 20 owned containers, 25 volumes, four networks, one unused old image,
the dedicated Android AVD and eight ignored build-output paths. Uninstalled
Bark's iOS bundles and shut down its shared iPhone; preserved Noah/Nitro apps,
all unrelated Docker container IDs and active Noah devices. All ownership,
absence and shutdown checks passed. Available disk increased by 52.96 GB to
88.26 GB. [Results](../validation/v030-cleanup.json) and
[release record](../validation/2026-10-19-v030.md) retain the evidence.
