# Prepared onchain sends: execution record

Engineering plan: [Noah items 14–16](../plans/20-noah-release-priorities.md).

Implemented signed, exact-fee send/drain previews, shared BDK reservations,
confirmed-only filtering, exact-byte broadcast/discard, typed BarkError codes and
accepted/failed/notAttempted results. Pending Lightning listings already expose
hash, amount and fee; migration guidance documents the amount check before resume.

Validation to date:

- Rust check and format pass. Three BDK/real-SQLite tests pass: construction
  failures and preparation leave all database/sidecar bytes unchanged; fees use
  signed many-input transactions; recorded bytes reopen exactly with no persisted
  transient locks.
- Full live Rust library suite: 88 passed, zero failures, including cancellation
  cleanup, existing schema-42 opening, balances and existing board/exit flows.
- Three additional live RegTest tests passed in 17.66s. Bitcoin Core observed
  transaction a5c1d9ccf9884f79bb0801ccead9e9a2f40de5d50ac98738786ee996bab802be:
  12 inputs, 105,000 recipient sats, 7,775 fee sats, 775 signed vB. Verified input
  identities/txid/fee/vsize, another handle, a running daemon, confirmed-only drain,
  unconfirmed inclusion, invalid rates including conversion-valid overflow, dust,
  unknown IDs, discard and a real SQLite trigger failure before submission.
- Failed-network retry regression passed through a transparent Bitcoin RPC proxy:
  an evicted old preparation cannot spend a newer reservation; retry after discard
  re-records it with a timestamp newer than eviction.
- Delayed real Ark cosigning regression passed with a separate Ark database:
  preparation waits for the whole board call and excludes its funding inputs.
  Supplied board PSBTs spending reservations are rejected before upstream execution.
- GPT-6 Astra high review found four real issues (boarding race, evicted retry,
  cancelled-result leak, fee multiplication overflow). All were verified/fixed;
  its second review approved with no remaining source-level blockers.

SDK compilation/packaged contracts and hosted CI are still pending. Do not count
Rust tests as Swift/Kotlin runtime acceptance. IDs are process-local; discard
cannot undo a transaction already recorded for broadcast/retry. Unknown upstream
error categories remain Other with their diagnostic chain. Physical devices and
external payment providers are outside this change's validation.
