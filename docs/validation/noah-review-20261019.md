# Noah parity independent review

Fable 5.1, High effort, reviewed PRs #1/#2 at combined head 249386e against
bbe22f3 on 2026-10-19. The reviewer inspected source and did not execute checks.
Task: noah-parity-fable51-high-review-r1-20261019.

## Validated findings and decisions

1. **Observer stops on transient SQLite contention (medium): fixed.** A real
   rollback-journal writer held BEGIN EXCLUSIVE across the 750ms poll. Before
   the repair, the next event was resyncRequired and observation terminated.
   BUSY/LOCKED now retry on the next tick; other errors retain resyncRequired.
   The regression checks databaseChanged after commit and an active observer.
2. **Validation ignores uncheckpointed WAL (low): fixed.** A real WAL database
   with committed, uncheckpointed rows was accepted using only the stale main
   file. Validation now rejects nonempty -wal/-journal siblings before/after
   copying, with an instruction to export a self-contained snapshot. It never
   opens or recovers the input through SQLite. Tests compare input/WAL bytes and
   verify that checkpointing permits validation, plus journal rejection.
3. **Queued events remain after stop (low): retained and documented.** This
   matches Nitro. Stop ends production and isActive becomes false; queued
   committed events drain before nil/null. A regression checks the queued
   initial event and the end of stream.
4. **10ms pause on each backup step (informational): retained.** The task
   explicitly requests this pause and a 30s continuous-busy timeout. It is not
   an overall elapsed deadline; active writes can extend backup duration.
5. **Zero sat/vB drain rate accepted (low): retained.** Nitro and pinned Bark
   accept zero through FeeRate::from_sat_per_vb. Adding a minimum would alter
   the requested upstream transaction validation and Nitro behavior. Broadcast
   acceptance still depends on the chain backend's relay policy.

The engineering repair plan precedes code in docs/plans/17-noah-native-parity.md.
Both regressions failed before their fixes. Final validation and release evidence
are recorded there and in the release validation report.

After repair: BARK_REGTEST=1 cargo test --locked passes all 74 tests (including
the three added regressions), and warnings-denied Clippy passes all targets.
