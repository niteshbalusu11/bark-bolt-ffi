use std::sync::Arc;
use tokio::sync::RwLock;

use bark::onchain::OnchainWalletTrait;
use log::info;

use crate::config::Config;
use crate::core::onchain::OnchainWallet as CoreOnchainWallet;
use crate::error::BarkError;
use crate::ffi::custom_onchain_wallet::{CallbackWalletAdapter, CustomOnchainWalletCallbacks};
use crate::ffi::db::open_sqlite_db;
use crate::ffi::runtime::run_async;
use crate::types::OnchainBalance;
use crate::Network;

/// BoltFFI-facing onchain wallet. Supports two backends:
/// - BDK (real onchain wallet via `bark::onchain::OnchainWallet`)
/// - Callback (foreign-language implementation)
pub struct OnchainWallet {
    inner: OnchainWalletInner,
}

enum OnchainWalletInner {
    Bdk(Arc<CoreOnchainWallet>),
    Callback(Arc<RwLock<CallbackWalletAdapter>>),
}

#[boltffi::export]
impl OnchainWallet {
    /// BDK-backed wallet. Opens the shared sqlite cache.
    pub async fn default(
        network: Network,
        mnemonic: String,
        config: Config,
        datadir: String,
    ) -> Result<Self, BarkError> {
        run_async(async move {
            let db = open_sqlite_db(&datadir)?;
            let core = CoreOnchainWallet::default(network, mnemonic, config, db).await?;
            Ok(Self {
                inner: OnchainWalletInner::Bdk(Arc::new(core)),
            })
        })
        .await
    }

    /// Callback-backed wallet for foreign-language implementations.
    pub fn custom(callbacks: Arc<dyn CustomOnchainWalletCallbacks>) -> Result<Self, BarkError> {
        info!("[ONCHAIN] Creating callback-based onchain wallet");
        let adapter = CallbackWalletAdapter::new(callbacks);
        Ok(Self {
            inner: OnchainWalletInner::Callback(Arc::new(RwLock::new(adapter))),
        })
    }

    pub async fn sync(&self) -> Result<u64, BarkError> {
        match &self.inner {
            OnchainWalletInner::Bdk(core) => {
                let core = core.clone();
                run_async(async move { core.sync().await }).await
            }
            OnchainWalletInner::Callback(_) => {
                info!("[ONCHAIN] Callback wallets manage their own sync");
                Ok(0)
            }
        }
    }

    /// Discover the wallet's pre-existing on-chain history. Run once after
    /// restoring a wallet from a mnemonic: `sync` only covers addresses this
    /// wallet instance has already revealed, so it never finds transactions
    /// made by a previous incarnation. Gap-limited full scan on esplora
    /// (`birthday_height` is ignored there), block scan from `birthday_height`
    /// on bitcoind. Returns the total balance in sats afterwards.
    pub async fn initial_scan(&self, birthday_height: Option<u32>) -> Result<u64, BarkError> {
        match &self.inner {
            OnchainWalletInner::Bdk(core) => {
                let core = core.clone();
                run_async(async move { core.initial_scan(birthday_height).await }).await
            }
            OnchainWalletInner::Callback(_) => {
                Err("initial_scan() not supported for callback wallets".into())
            }
        }
    }

    pub async fn balance(&self) -> Result<OnchainBalance, BarkError> {
        match &self.inner {
            OnchainWalletInner::Bdk(core) => {
                let core = core.clone();
                run_async(async move { core.balance().await }).await
            }
            OnchainWalletInner::Callback(adapter) => {
                let adapter = adapter.clone();
                run_async(async move {
                    let a = adapter.read().await;
                    let amount = a.balance().await;
                    Ok(OnchainBalance {
                        confirmed_sats: amount.to_sat(),
                        pending_sats: 0,
                        total_sats: amount.to_sat(),
                    })
                })
                .await
            }
        }
    }

    pub async fn new_address(&self) -> Result<String, BarkError> {
        match &self.inner {
            OnchainWalletInner::Bdk(core) => {
                let core = core.clone();
                run_async(async move { core.new_address().await }).await
            }
            OnchainWalletInner::Callback(_) => {
                Err("new_address() not supported for callback wallets".into())
            }
        }
    }

    pub async fn send(
        &self,
        address: String,
        amount_sats: u64,
        fee_rate_sat_per_vb: u64,
    ) -> Result<String, BarkError> {
        match &self.inner {
            OnchainWalletInner::Bdk(core) => {
                let core = core.clone();
                run_async(async move { core.send(address, amount_sats, fee_rate_sat_per_vb).await })
                    .await
            }
            OnchainWalletInner::Callback(_) => {
                Err("send() not supported for callback wallets".into())
            }
        }
    }

    /// Mark a wallet-known transaction as evicted from the mempool, so its
    /// inputs return to coin selection immediately. Only for a tx that has
    /// definitively been superseded on-chain (e.g. an RBF-replaced exit CPFP);
    /// evicting a still-in-flight tx invites a self-inflicted double-spend.
    pub async fn evict_tx(&self, txid: String) -> Result<(), BarkError> {
        match &self.inner {
            OnchainWalletInner::Bdk(core) => {
                let core = core.clone();
                run_async(async move { core.evict_tx(txid).await }).await
            }
            OnchainWalletInner::Callback(adapter) => {
                let adapter = adapter.clone();
                run_async(async move {
                    let txid = txid
                        .parse::<bitcoin::Txid>()
                        .map_err(|e| BarkError::from(format!("invalid txid: {}", e)))?;
                    let mut a = adapter.write().await;
                    a.evict_tx(txid).await.map_err(BarkError::from)
                })
                .await
            }
        }
    }

    /// Current chain tip height from the wallet's chain source.
    pub async fn tip_height(&self) -> Result<u32, BarkError> {
        match &self.inner {
            OnchainWalletInner::Bdk(core) => {
                let core = core.clone();
                run_async(async move { core.tip_height().await }).await
            }
            OnchainWalletInner::Callback(_) => {
                Err("tip_height() not supported for callback wallets".into())
            }
        }
    }

    /// Cached network fee-rate estimates from the wallet's chain source.
    pub async fn fee_rates(&self) -> Result<crate::types::FeeRates, BarkError> {
        match &self.inner {
            OnchainWalletInner::Bdk(core) => {
                let core = core.clone();
                run_async(async move { core.fee_rates().await }).await
            }
            OnchainWalletInner::Callback(_) => {
                Err("fee_rates() not supported for callback wallets".into())
            }
        }
    }

    /// Every wallet transaction with fee, balance change, confirmation and
    /// CPFP flag. Requires a prior `sync` to be meaningful.
    pub async fn transactions(&self) -> Result<Vec<crate::types::WalletTransaction>, BarkError> {
        match &self.inner {
            OnchainWalletInner::Bdk(core) => {
                let core = core.clone();
                run_async(async move { core.transactions().await }).await
            }
            OnchainWalletInner::Callback(_) => {
                Err("transactions() not supported for callback wallets".into())
            }
        }
    }

    /// The wallet's unspent outputs. Requires a prior `sync` to be meaningful.
    pub async fn utxos(&self) -> Result<Vec<crate::types::OnchainUtxo>, BarkError> {
        match &self.inner {
            OnchainWalletInner::Bdk(core) => {
                let core = core.clone();
                run_async(async move { core.utxos().await }).await
            }
            OnchainWalletInner::Callback(_) => {
                Err("utxos() not supported for callback wallets".into())
            }
        }
    }
}

impl OnchainWallet {
    pub(crate) fn inner_dyn(&self) -> Arc<RwLock<dyn OnchainWalletTrait>> {
        match &self.inner {
            OnchainWalletInner::Bdk(v) => v.inner(),
            OnchainWalletInner::Callback(v) => v.clone(),
        }
    }
}
