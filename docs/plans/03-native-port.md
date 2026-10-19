# Tasks 03–06 engineering plan: production native adapter

Paths: Cargo.toml/lock, src/lib.rs, src/types.rs, src/config.rs, src/error.rs,
src/core/{wallet,notification}.rs and src/muniffi (rename to src/ffi).

- Rename the package/library to bark-bolt-ffi/bark_bolt_ffi. Keep Bark 0.7.1.
  Make the native SQLite/BDK/TLS/Tokio settings unconditional; remove the unused
  browser feature and source, while retaining Serde for actual JSON conversions.
- Use `BarkError { message: String }`, formatting anyhow cause chains on entry.
  Convert unexpected foreign callback errors into this same type. Preserve
  existing method behavior and all ten transaction-validation callbacks.
- Export concrete Wallet/OnchainWallet/NotificationHolder objects, keeping
  internal Arc ownership. Make WalletOpenArgs contain value fields only; expose
  open without an onchain wallet and open_with_onchain with a borrowed object.
- Translate record and parameter defaults to BoltFFI attributes; add Copy only
  to scalar records where the generated direct codec requires it.
- Notification methods borrow self. A Drop guard clears both the cancel sender
  and wait flag, including foreign future cancellation. The Tokio offload helper
  keeps its existing detached-task semantics. Keep daemon stop on Wallet drop.
- Drop replaced logger callbacks outside the lock; native-only consumers no
  longer need the intentional React Native runtime-reload leak.

Validation: compare docs/api-baseline.json to the actual new exported names,
with explicit mappings for factories; run all 52 baseline tests plus focused
notification/lifetime cases; strict-generate both hosts and compile/run real
SDK consumers. Never remove PSBT output/signature checks or datadir locking to
make macro expansion succeed. Package module names are BarkBoltFFI and
io.github.niteshbalusu11.bark; breaking API changes are documented.

Actual device prerequisite: explicitly enable bark/bitcoind-rpc; bark/native does
not include it when default-features is false. The real mobile chain-source open
failed without this feature, so app smoke tests must guard it.
