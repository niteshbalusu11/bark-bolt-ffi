# CI caches shared from main

## Findings (2026-10-22)

Full main run [39874123211](https://github.com/smolcars/bark-bolt-ffi/actions/runs/39874123211):

| Job | Total | Native SDK build |
| --- | --- | --- |
| android | 34.5 min | 23.7 min (`build-android --release`, three ABIs plus the host JVM library) |
| apple | 16.9 min | 14.1 min (`build-ios --release`, five slices) |
| rust | 3.9 min | Clippy 1.5 min, tests 1.6 min |

- The android and rust jobs logged `Cache not found` for their exact keys and
  for their `android-Linux-` / `rust-Linux-` prefixes. Apple restored an
  October 20 main entry.
- Blacksmith runners transparently redirect `actions/cache` (and actions built
  on it) to Blacksmith's colocated cache: 25 GB per repo, LRU eviction, entries
  scoped per branch/tag like GitHub's
  ([docs](https://docs.blacksmith.sh/blacksmith-caching/dependencies-actions.md)).
  GitHub's own cache API lists only three stale `v0.4.0` tag entries.
- Every PR and tag run saves about 9 GB (android 5.3 GB of whole `target/` plus
  Gradle, apple 2.7 GB, rust 0.9 GB). Those entries are readable only by that
  PR or tag, and they evict main's entries, which every PR could read.
- Whole-`target/` caches also keep stale dependency versions and our own crate
  forever.

## Engineering plan

Affected paths: `.github/workflows/ci.yml` (rust, android and apple cache
steps), this plan, the execution checklist.

- **Rust** (all three jobs): replace the `actions/cache` step with
  `Swatinem/rust-cache` v2.9.3 (`d1527c8f67ef9af357290475d4af4b0fab8f12e0`,
  verified from the release tag), with
  `save-if: ${{ github.ref == 'refs/heads/main' }}`.
  - It caches `~/.cargo` and dependency artifacts only, not workspace crates
    or incremental data, and prunes unused or week-old artifacts.
  - Keys are per job, rustc and Cargo.lock. On a lockfile change it restores
    the previous lockfile's entry.
  - It is built on `actions/cache`, so it uses the Blacksmith backend.
- **Gradle** (android): split into `actions/cache/restore` everywhere and
  `actions/cache/save` only on main, with the existing pinned `actions/cache`
  v6.1.0 commit. The key is the Gradle scripts plus wrapper properties.
- PRs and tags restore main's entries and never save. Saving only on success
  keeps a failed main run from storing a partial cache.

Invariants: no build, test, artifact or release-staging step changes. Published
SDKs still come only from a green full-matrix main run, and the staging recipe
does not read caches.

Validation: actionlint on the workflow; PR CI green. Then merge, let the first
main run save, and rerun that main run to measure warm restore and build times
against the cold numbers above. Record outcomes here.
