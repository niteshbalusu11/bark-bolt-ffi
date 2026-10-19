use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use futures_util::StreamExt;
use tokio::sync::oneshot;

use crate::error::BarkError;
use crate::types::WalletNotification;

/// Pull-based notification handle exposed over FFI.
///
/// Obtain via `Wallet::notifications()`. Call `next_notification()` in a loop
/// to receive events. Call `cancel_next_notification_wait()` to unblock a
/// pending wait without destroying the underlying stream.
///
/// Each call to `Wallet::notifications()` creates an independent stream backed
/// by a new broadcast receiver — existing holders are unaffected.
///
/// This holder is intended for a single consumer loop. Concurrent calls to
/// `next_notification()` on the same holder are not supported and will return
/// `None` immediately.
pub struct NotificationHolder {
    /// The bark notification stream. Held in a tokio Mutex because we need to
    /// hold it across the `.await` point inside `next_notification()`.
    stream: tokio::sync::Mutex<bark::NotificationStream>,

    /// Sender half of the per-wait cancellation one-shot channel.
    /// Replaced on every `next_notification()` call, so cancellation is scoped
    /// to exactly one wait cycle. A std Mutex suffices here because
    /// `cancel_next_notification_wait()` is synchronous and never held across
    /// an await point.
    cancel: Mutex<Option<oneshot::Sender<()>>>,

    /// Guards against concurrent `next_notification()` calls on the same holder.
    /// Reset via a RAII guard so it is always cleared even if the async body panics.
    wait_in_progress: AtomicBool,
}

/// RAII guard that clears `wait_in_progress` on drop, including on panic.
struct WaitGuard<'a> {
    wait_in_progress: &'a AtomicBool,
    cancel: &'a Mutex<Option<oneshot::Sender<()>>>,
}

impl Drop for WaitGuard<'_> {
    fn drop(&mut self) {
        let sender = self.cancel.lock().unwrap().take();
        self.wait_in_progress.store(false, Ordering::Release);
        drop(sender);
    }
}

impl NotificationHolder {
    pub(crate) fn new(wallet: &bark::Wallet) -> Self {
        Self {
            stream: tokio::sync::Mutex::new(wallet.subscribe_notifications()),
            cancel: Mutex::new(None),
            wait_in_progress: AtomicBool::new(false),
        }
    }
}

#[boltffi::export]
impl NotificationHolder {
    /// Wait for the next wallet notification.
    ///
    /// Returns `None` when:
    /// - `cancel_next_notification_wait()` was called while this was pending
    ///   (cancellation only affects the current wait; the stream lives on)
    /// - The wallet's notification source was shut down permanently
    ///
    /// Returns an error if called concurrently on the same holder.
    ///
    /// After a cancellation this method can be called again normally — the
    /// underlying `NotificationStream` is preserved in `self.stream` and a
    /// fresh per-wait cancel channel is created on every entry.
    pub async fn next_notification(&self) -> Result<Option<WalletNotification>, BarkError> {
        // Enforce single-consumer: reject concurrent calls with an explicit error.
        if self
            .wait_in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("next_notification() called concurrently on the same holder".into());
        }

        // RAII guard resets wait_in_progress on exit, even on panic.
        let _guard = WaitGuard {
            wait_in_progress: &self.wait_in_progress,
            cancel: &self.cancel,
        };

        // Create a fresh one-shot cancel channel for this specific wait cycle.
        // Once consumed — either by cancellation or by next_notification
        // returning — it is discarded. The next call creates a new channel.
        let (tx, rx) = oneshot::channel::<()>();
        {
            let mut cancel = self.cancel.lock().unwrap();
            debug_assert!(cancel.is_none(), "cancel slot should be empty on entry");
            *cancel = Some(tx);
        }

        let result = {
            let mut stream = self.stream.lock().await;
            tokio::select! {
                // Normal case: a notification arrived on the stream.
                notification = stream.next() => {
                    notification.map(WalletNotification::from)
                }
                // Cancellation case: cancel_next_notification_wait() sent on
                // `tx`. The stream itself is completely unaffected — only this
                // wait is interrupted. The next call to next_notification()
                // acquires the lock again and waits from where the stream left off.
                _ = rx => None,
            }
        };

        // Clear the cancel slot. Safe: wait_in_progress ensures we are the
        // sole active waiter, so no concurrent call can own this slot.
        *self.cancel.lock().unwrap() = None;

        Ok(result)
    }

    /// Cancel the currently pending `next_notification()` wait.
    ///
    /// Causes a blocked `next_notification()` to return `None`.
    /// Has no effect if no wait is currently active.
    ///
    /// This does NOT destroy the underlying `NotificationStream`; a subsequent
    /// call to `next_notification()` will work normally and wait for new events.
    pub fn cancel_next_notification_wait(&self) {
        // Waking a foreign continuation can resume it synchronously on Android.
        let sender = self.cancel.lock().unwrap().take();
        if let Some(tx) = sender {
            let _ = tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropped_wait_clears_cancel_before_releasing_wait_flag() {
        let active = AtomicBool::new(true);
        let (tx, mut rx) = oneshot::channel();
        let slot = Mutex::new(Some(tx));
        drop(WaitGuard {
            wait_in_progress: &active,
            cancel: &slot,
        });
        assert!(!active.load(Ordering::Acquire));
        assert!(slot.lock().unwrap().is_none());
        assert_eq!(rx.try_recv(), Err(oneshot::error::TryRecvError::Closed));
    }
}
