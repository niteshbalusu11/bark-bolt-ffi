//! Throwaway native compatibility proof, independent of Bark and real wallet data.

use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use tokio::sync::{broadcast, oneshot};

static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("bark-boltffi-proof")
        .build()
        .unwrap()
});
static LIVE_WALLETS: AtomicU32 = AtomicU32::new(0);

async fn run_async<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> T {
    RUNTIME.spawn(future).await.unwrap()
}

#[boltffi::error]
#[derive(Clone, Debug)]
pub struct WalletError {
    pub message: String,
}

impl std::fmt::Display for WalletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for WalletError {}

impl From<boltffi::UnexpectedFfiCallbackError> for WalletError {
    fn from(error: boltffi::UnexpectedFfiCallbackError) -> Self {
        Self {
            message: error.message().to_owned(),
        }
    }
}

#[boltffi::data]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Balance {
    pub confirmed_sats: u64,
    pub pending_sats: u64,
}

#[boltffi::data]
#[derive(Clone, Debug)]
pub struct Destination {
    pub address: String,
    pub amount_sats: u64,
}

#[boltffi::data]
#[derive(Clone, Debug)]
pub struct WalletOpenArgs {
    pub datadir: String,
    #[boltffi::default(true)]
    pub run_daemon: bool,
    #[boltffi::default(None)]
    pub label: Option<String>,
}

#[boltffi::data]
#[derive(Clone, Debug)]
pub enum WalletNotification {
    MovementCreated { destinations: Vec<Destination> },
    ChannelLagging,
}

#[boltffi::export]
pub trait CustomOnchainWalletCallbacks: Send + Sync {
    fn get_balance(&self) -> Result<u64, WalletError>;
    fn prepare_tx(&self, destinations: Vec<Destination>) -> Result<String, WalletError>;
    fn sync(&self) -> Result<(), WalletError>;
}

#[boltffi::export]
pub trait BarkLogger: Send + Sync {
    fn log(&self, message: String);
}

pub struct OnchainWallet {
    callbacks: Arc<dyn CustomOnchainWalletCallbacks>,
}

#[boltffi::export]
impl OnchainWallet {
    pub fn custom(callbacks: Arc<dyn CustomOnchainWalletCallbacks>) -> Self {
        Self { callbacks }
    }

    pub fn balance(&self) -> Result<u64, WalletError> {
        self.callbacks.get_balance()
    }
}

pub struct Wallet {
    callbacks: Option<Arc<dyn CustomOnchainWalletCallbacks>>,
    logger: Arc<dyn BarkLogger>,
    notifications: broadcast::Sender<WalletNotification>,
    label: Option<String>,
    daemon_requested: bool,
}

impl Wallet {
    async fn build(
        args: WalletOpenArgs,
        logger: Arc<dyn BarkLogger>,
        callbacks: Option<Arc<dyn CustomOnchainWalletCallbacks>>,
    ) -> Result<Self, WalletError> {
        run_async(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            if args.datadir == "fail" {
                return Err(WalletError {
                    message: "opening wallet: invalid datadir".into(),
                });
            }
            logger.log("opened on Tokio worker".into());
            LIVE_WALLETS.fetch_add(1, Ordering::SeqCst);
            let (notifications, _) = broadcast::channel(8);
            Ok(Self {
                callbacks,
                logger,
                notifications,
                label: args.label,
                daemon_requested: args.run_daemon,
            })
        })
        .await
    }
}

#[boltffi::export]
impl Wallet {
    pub async fn open(
        args: WalletOpenArgs,
        logger: Arc<dyn BarkLogger>,
    ) -> Result<Self, WalletError> {
        Self::build(args, logger, None).await
    }

    pub async fn open_with_onchain(
        args: WalletOpenArgs,
        logger: Arc<dyn BarkLogger>,
        onchain: &OnchainWallet,
    ) -> Result<Self, WalletError> {
        Self::build(args, logger, Some(onchain.callbacks.clone())).await
    }

    pub async fn balance(&self) -> Result<Balance, WalletError> {
        let callbacks = self.callbacks.clone();
        let logger = self.logger.clone();
        run_async(async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            logger.log("balance on Tokio worker".into());
            let confirmed_sats = match callbacks {
                Some(callbacks) => callbacks.get_balance()?,
                None => 0,
            };
            Ok(Balance {
                confirmed_sats,
                pending_sats: 0,
            })
        })
        .await
    }

    pub async fn sync(&self) -> Result<(), WalletError> {
        let callbacks = self.callbacks.clone();
        run_async(async move {
            if let Some(callbacks) = callbacks {
                callbacks.sync()?;
            }
            Ok(())
        })
        .await
    }

    pub async fn prepare_tx(&self, destinations: Vec<Destination>) -> Result<String, WalletError> {
        let callbacks = self.callbacks.clone();
        run_async(async move {
            callbacks
                .ok_or_else(|| WalletError {
                    message: "no onchain wallet".into(),
                })?
                .prepare_tx(destinations)
        })
        .await
    }

    pub fn label(&self) -> Option<String> {
        self.label.clone()
    }

    pub fn daemon_requested(&self) -> bool {
        self.daemon_requested
    }

    pub fn notifications(&self) -> NotificationHolder {
        NotificationHolder {
            stream: tokio::sync::Mutex::new(self.notifications.subscribe()),
            cancel: Mutex::new(None),
            wait_in_progress: AtomicBool::new(false),
        }
    }

    pub fn emit(&self, notification: WalletNotification) {
        let _ = self.notifications.send(notification);
    }
}

impl Drop for Wallet {
    fn drop(&mut self) {
        LIVE_WALLETS.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct NotificationHolder {
    stream: tokio::sync::Mutex<broadcast::Receiver<WalletNotification>>,
    cancel: Mutex<Option<oneshot::Sender<()>>>,
    wait_in_progress: AtomicBool,
}

struct WaitGuard<'a>(&'a NotificationHolder);

impl Drop for WaitGuard<'_> {
    fn drop(&mut self) {
        // Foreign cancellation skips the code after await; clear both slots on drop.
        *self.0.cancel.lock().unwrap() = None;
        self.0.wait_in_progress.store(false, Ordering::Release);
    }
}

#[boltffi::export]
impl NotificationHolder {
    pub async fn next_notification(&self) -> Result<Option<WalletNotification>, WalletError> {
        if self
            .wait_in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(WalletError {
                message: "concurrent notification wait".into(),
            });
        }
        let _guard = WaitGuard(self);
        let (tx, rx) = oneshot::channel();
        *self.cancel.lock().unwrap() = Some(tx);
        let mut stream = self.stream.lock().await;
        tokio::select! {
            notification = stream.recv() => match notification {
                Ok(notification) => Ok(Some(notification)),
                Err(broadcast::error::RecvError::Closed) => Ok(None),
                Err(broadcast::error::RecvError::Lagged(_)) => Ok(Some(WalletNotification::ChannelLagging)),
            },
            _ = rx => Ok(None),
        }
    }

    pub fn cancel_next_notification_wait(&self) {
        if let Some(tx) = self.cancel.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }

    pub fn is_waiting(&self) -> bool {
        self.wait_in_progress.load(Ordering::Acquire)
    }
}

#[boltffi::export]
pub fn live_wallets() -> u32 {
    LIVE_WALLETS.load(Ordering::SeqCst)
}

#[boltffi::export]
pub fn echo_amount(amount: u64) -> u64 {
    amount
}
