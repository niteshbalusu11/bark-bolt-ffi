use std::sync::Arc;
use tokio::sync::RwLock;

use bark::chain::{ChainSource, ChainSourceSpec};
use bark::onchain::{OnchainWallet as BarkOnchainWallet, OnchainWalletTrait};
use bark::persist::BarkPersister;
use bip39::Mnemonic;

use anyhow::Context;
use log::info;

use crate::config::Config;
use crate::core::parse_address;
use crate::error::BarkError;
use crate::types::{self, OnchainBalance};
use crate::Network;

/// BDK-based onchain Bitcoin wallet for boarding and exits.
///
/// Core type, shared between uniffi and wasm bindings. Holds only the
/// BDK case — callback-backed wallets are a uniffi-only concept layered
/// on top of the core inner `bark::Wallet`.
pub struct OnchainWallet {
    wallet: Arc<RwLock<BarkOnchainWallet>>,
    chain: Arc<ChainSource>,
    /// For [`Self::send`]; neither the bdk wallet nor the trait exposes one.
    network: bitcoin::Network,
}

impl OnchainWallet {
    pub async fn default(
        network: Network,
        mnemonic: String,
        config: Config,
        db: Arc<dyn BarkPersister>,
    ) -> Result<Self, BarkError> {
        let mnemonic = Mnemonic::parse(mnemonic.trim()).context("invalid mnemonic")?;

        let seed = mnemonic.to_seed("");
        let network = network.into();

        info!(
            "[ONCHAIN] Creating onchain wallet for {:?} network",
            network
        );

        let onchain = BarkOnchainWallet::load_or_create(network, seed, db.clone()).await?;

        let chain_spec = if let Some(url) = config.bitcoind_address.as_ref() {
            use bark_bitcoin_ext::rpc::Auth;
            let auth = if let Some(cookie) = config.bitcoind_cookiefile {
                Auth::CookieFile(std::path::PathBuf::from(cookie))
            } else if let (Some(user), Some(pass)) = (config.bitcoind_user, config.bitcoind_pass) {
                Auth::UserPass(user, pass)
            } else {
                Auth::None
            };
            info!("[ONCHAIN] Using Bitcoin Core RPC at {}", url);
            // ZMQ block notifications are configurable upstream via
            // `Config::bitcoind_zmq_address`; the FFI config does not expose it
            // yet, so the chain tip is polled instead.
            ChainSourceSpec::Bitcoind {
                url: url.clone(),
                auth,
                zmq: None,
            }
        } else if let Some(url) = config.esplora_address {
            info!("[ONCHAIN] Using Esplora at {}", url);
            ChainSourceSpec::Esplora { url }
        } else {
            return Err(anyhow::anyhow!(
                "Config must specify either esplora_address or bitcoind_address"
            )
            .into());
        };

        let chain = Arc::new(
            ChainSource::new(chain_spec, network, None)
                .await
                .context("Failed to create chain source")?,
        );

        Ok(Self {
            wallet: Arc::new(RwLock::new(onchain)),
            chain,
            network,
        })
    }

    pub async fn sync(&self) -> Result<u64, BarkError> {
        let mut w = self.wallet.write().await;
        w.sync(&self.chain).await.context("Sync failed")?;
        let balance = w.balance();
        Ok(balance.total().to_sat())
    }

    /// Discover the wallet's pre-existing on-chain history.
    ///
    /// `sync` only checks addresses this wallet instance has already revealed,
    /// so a wallet restored from a mnemonic never finds transactions made by
    /// its previous incarnation. This runs bark's initial scan instead: a
    /// gap-limited full scan on esplora (`birthday_height` is ignored there),
    /// or a block scan from `birthday_height` on bitcoind. Run it once after
    /// restoring; returns the total balance in sats afterwards.
    pub async fn initial_scan(&self, birthday_height: Option<u32>) -> Result<u64, BarkError> {
        let mut w = self.wallet.write().await;
        let balance = w
            .initial_wallet_scan(&self.chain, birthday_height)
            .await
            .context("Initial wallet scan failed")?;
        Ok(balance.to_sat())
    }

    pub async fn balance(&self) -> Result<OnchainBalance, BarkError> {
        let w = self.wallet.write().await;
        Ok(w.balance().into())
    }

    pub async fn new_address(&self) -> Result<String, BarkError> {
        let mut w = self.wallet.write().await;
        let addr = w.address().await.context("Failed to generate address")?;
        Ok(addr.to_string())
    }

    pub async fn send(
        &self,
        address: String,
        amount_sats: u64,
        fee_rate_sat_per_vb: u64,
    ) -> Result<String, BarkError> {
        let addr = parse_address(&address, self.network)?;

        let amount = bitcoin::Amount::from_sat(amount_sats);

        let fee_rate =
            bitcoin::FeeRate::from_sat_per_vb(fee_rate_sat_per_vb).context("Invalid fee rate")?;

        let mut w = self.wallet.write().await;
        let txid = w
            .send(&self.chain, addr, amount, fee_rate)
            .await
            .context("Send failed")?;

        Ok(txid.to_string())
    }

    /// Mark a wallet-known transaction as evicted from the mempool, so its
    /// inputs return to coin selection immediately instead of waiting for the
    /// sync eviction grace period. Only call this on a tx that has definitively
    /// been superseded on-chain (e.g. an exit CPFP that was RBF-replaced);
    /// evicting a still-in-flight tx invites a self-inflicted double-spend.
    pub async fn evict_tx(&self, txid: String) -> Result<(), BarkError> {
        let txid = txid.parse::<bitcoin::Txid>().context("invalid txid")?;
        let mut w = self.wallet.write().await;
        w.evict_tx(txid).await.context("Evict tx failed")?;
        Ok(())
    }

    /// Current chain tip height, from the wallet's chain source (cached
    /// upstream with a short TTL).
    pub async fn tip_height(&self) -> Result<u32, BarkError> {
        let height = self.chain.tip().await.context("Failed to fetch tip")?;
        Ok(height)
    }

    /// Cached network fee-rate estimates from the wallet's chain source.
    pub async fn fee_rates(&self) -> Result<types::FeeRates, BarkError> {
        Ok(self.chain.fee_rates().await.into())
    }

    /// Every wallet transaction with fee, balance change, confirmation and
    /// CPFP flag. Requires a prior `sync` to be meaningful.
    pub async fn transactions(&self) -> Result<Vec<types::WalletTransaction>, BarkError> {
        let w = self.wallet.read().await;
        let infos = w
            .list_transaction_infos()
            .context("Failed to list transactions")?;
        Ok(infos.iter().map(Into::into).collect())
    }

    /// The wallet's unspent outputs. Requires a prior `sync` to be meaningful.
    pub async fn utxos(&self) -> Result<Vec<types::OnchainUtxo>, BarkError> {
        let w = self.wallet.read().await;
        Ok(w.utxos().iter().map(Into::into).collect())
    }

    pub(crate) fn inner(&self) -> Arc<RwLock<BarkOnchainWallet>> {
        self.wallet.clone()
    }
}
