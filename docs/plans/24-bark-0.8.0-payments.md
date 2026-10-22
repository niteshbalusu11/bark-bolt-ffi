# Bark 0.8.0, Lightning send options, payment requests and BIP 321 (v0.6.0)

## Upstream delta: bfbe6b7 → lib-0.8.0 (959a489f, 2026-10-22)

45 commits. Lightning send options (`LightningSendOptions`, `*_with`, #2582) and
payment-request parsing (`parse_payment_request`, `estimate_payment_fee(s)`,
`send_payment(_with)`, `bip321_uri`) already existed at bfbe6b7; they were never
wrapped. 0.8.0 itself adds:

- #2610 movement ids: `send_arkoor_payment` returns `MovementId`;
  `send_onchain`, `offboard_all`, `offboard_vtxos` return `OffboardResult
  {txid, movement_id}`; `PaymentInitOutput::Ark(MovementId)`;
  `BarkPersister::get_movement_id_by_action` (implemented by Bark's SQLite
  client; this crate does not implement the trait).
- Finished exits are no longer re-finished on every sync (stable `completed_at`
  and exit history), `finish_movement` is idempotent, arkoor sends no longer
  return early when a sync drives the same action, recovery pages with a VTXO
  cursor, VTXO registration batches at 1000.
- No Bark SQLite migration in the range: persistence stays compatible.

Server: `niteshbalusu/captaind:nightly-2026-10-22` reports
`0.7.1-dev+5d9c4eda`. Between 5d9c4eda and 959a489f the server differs only in a
telemetry version allow-list and a webpki-roots bump, so the image is the 0.8.0
server for protocol purposes. Its new config key defaults; `captaind.toml` is
unchanged. Retry-for support (#2580) predates both.

## Engineering plan

Affected paths: Cargo.toml/lock (three Bark crates → 959a489f), src/types.rs,
src/error.rs, src/core/wallet.rs, src/ffi/wallet.rs, regtest/compose.yaml
(captaind and Bark CLI digests), tests/prepared_{board,onchain}.rs (port env),
new tests/payments.rs, tests/swift and tests/kotlin contracts, both example
apps' send path, docs/api-baseline.json, README, docs/migration.md, Gradle
version, execution checklist.

Interface (breaking, v0.6.0):

- `LightningSendOptions { wait = false, retryForSecs: UInt32? = nil }` mirrors
  `bark::LightningSendOptions`. `payLightningInvoice/Offer/Address`, `payLnurl`
  replace `wait: Bool` with `options: LightningSendOptions? = nil` and call the
  upstream `*_with` methods. The server caps `retryForSecs` at its maximum;
  0 makes a single attempt.
- `sendArkoorPayment` returns the movement id (`UInt32`). `sendOnchain` returns
  `OffboardResult`; `OffboardResult` gains `movementId`.
- `parsePaymentRequest(input)` → `PaymentRequest {amountSats?, label?, message?,
  options, defaultOption?}`. Each `PaymentOption` has the existing
  `PaymentMethod {methodType, value}` record, typed `errors:
  [PaymentMethodParsingError]` (networkMismatch, invalidArkAddress(typed
  ArkAddressError), amountMismatch{expectedSats, gotSats}, invalidAmount,
  unsupported), `requiresAmount` and `supportsComment`. `defaultOption` is
  upstream's choice (Ark, Lightning, onchain; error-free only). A string that is
  no payment request fails with `BarkErrorCode.invalidPaymentRequest`.
- `estimatePaymentFee(method, amountSats)` and `estimatePaymentFees(request,
  amountSats?)` → `[PaymentMethodFee {method, fee}]`, cheapest first, using
  upstream's estimators.
- `sendPayment(method, amountSats?, comment?, options?)` → `PaymentResult`:
  `onchain{txid, movementId}`, `lightning{status}`, `ark{movementId}`. Lightning
  and Ark dispatch through upstream `send_payment_with`. Onchain uses the
  wallet's `send_onchain` directly, because upstream's dispatcher drops the
  movement id; the network check and amount requirement are identical.
- `bip321Uri(options: Bip321UriOptions? = nil)` → `Bip321Uri {uri, arkAddress?,
  onchainAddress?, invoice?}` via upstream's builder on a cloned `bark::Wallet`
  handle (an `Arc`). Ark addresses and invoices can call the server, so when
  either is requested the onchain address is derived first under a short
  onchain write lock (like `OnchainWallet.newAddress`) and added afterwards
  where upstream puts it (mainnet address, `tb` elsewhere). Onchain-only URIs
  pass the locked onchain wallet to the builder; they make no server call.
  Without an onchain wallet the onchain destination is skipped, as upstream
  does; BOLT11 needs an amount; options leaving no destination are rejected.
- `debugInfo()` → `DebugInfo {network, mailboxId, vtxoXpub}` (upstream
  `debug_info`).

Ownership/concurrency: all new calls go through the existing `Wallet::run`
Tokio offload. `bip321Uri` holds the onchain write lock only for local address
derivation, never across a server call; it never takes the prepared-onchain
operation gate because it neither selects nor reserves inputs. No new
persisted state.

Review refinement: upstream `parse_payment_request` unwraps BOLT12 offer
amounts (panics on fiat-priced offers) and debug-asserts a non-empty option
list. `run_async` re-raises task panics across the FFI, so the wrapper catches
the parse panic and rejects empty requests, both as `invalidPaymentRequest`.
Passing an amount for an invoice that has one lets Bark pay up to twice the
invoice (BOLT 4); the examples and docs send the request's own amount and ask
only when `requiresAmount` is set.

Invariants: no wallet logic, fee algorithm or transaction validation is
replaced; parse/estimate/send/build call the upstream methods. Per-option
parse problems are data, not errors. Typed errors never classify a database
failure as an invalid payment request: the network is read before parsing.

Validation: fmt, all-target Clippy `-D warnings`, offline `cargo test`; live
RegTest Rust test for parse (Ark, BOLT11, BIP 321 with Ark+onchain+BOLT11,
mainnet address, amount mismatch, garbage), fee estimates, `sendPayment` Ark
movement id vs history, onchain txid/movement id, BOLT11 with options, raw
`payLightningInvoice` with `retryForSecs`, `bip321Uri` round trip and
`debugInfo`. Confirm in captaind's JSON span logs that a payment's `retry_for`
equals the requested value and that an oversized value is capped. Regenerate both SDKs strictly,
check API parity, run Swift and Kotlin live contracts, build and run both
example apps' RegTest acceptance on a task-owned simulator/emulator. Hosted CI
must pass on the PR and main; publish v0.6.0 from the main CI artifacts as in
v0.5.0, verify fresh downloads and the public Swift consumer, then notify the
Noah rewrite thread.

Environment: other agents' `noah-bark08-*` and `scripts-*` stacks, the booted
shared iPhone simulator and its runner stay untouched. Live checks run on an
isolated Compose project (`bark-bolt-ffi-v060`) with its own ports and volumes;
the repository's existing `bark-bolt-ffi` stack is left as is.

Remaining limits known before implementation: LNURL and Lightning Address
sends need an LNURL server, which the RegTest stack lacks; they get parse
coverage only. Upstream logs, but does not return, Lightning failure reasons.
