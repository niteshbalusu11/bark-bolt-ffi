//! Nitro v0.0.144 committed-database observer, adapted to a cancellable async holder.
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};
use tokio::sync::{mpsc, oneshot};

use super::notification::WaitGuard;
use crate::BarkError;

#[boltffi::data]
pub struct StateChangeEvent {
    pub sequence: u64,
    /// initial, databaseChanged, or resyncRequired (same spellings as Nitro).
    pub reason: String,
}

pub struct StateChangeSubscription {
    receiver: tokio::sync::Mutex<mpsc::UnboundedReceiver<StateChangeEvent>>,
    cancel: Mutex<Option<oneshot::Sender<()>>>,
    wait_in_progress: AtomicBool,
    active: Arc<AtomicBool>,
    stop_signal: Arc<(Mutex<bool>, Condvar)>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

fn data_version(connection: &Connection) -> rusqlite::Result<i64> {
    connection.query_row("PRAGMA data_version", [], |row| row.get(0))
}

impl StateChangeSubscription {
    pub(crate) fn new(path: &Path) -> anyhow::Result<Self> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        connection.busy_timeout(Duration::from_millis(100))?;
        let mut version = data_version(&connection)?;
        let (sender, receiver) = mpsc::unbounded_channel();
        sender.send(StateChangeEvent {
            sequence: 0,
            reason: "initial".into(),
        })?;
        let active = Arc::new(AtomicBool::new(true));
        let stop_signal = Arc::new((Mutex::new(false), Condvar::new()));
        let worker_active = active.clone();
        let worker_stop = stop_signal.clone();
        let worker = std::thread::Builder::new()
            .name("bark-state-changes".into())
            .spawn(move || {
                let mut sequence = 0_u64;
                loop {
                    let (lock, signal) = &*worker_stop;
                    let (stopped, _) = signal
                        .wait_timeout_while(
                            lock.lock().unwrap(),
                            Duration::from_millis(750),
                            |stopped| !*stopped,
                        )
                        .unwrap();
                    if *stopped {
                        break;
                    }
                    drop(stopped);
                    let reason = match data_version(&connection) {
                        Ok(next) if next != version => {
                            version = next;
                            "databaseChanged"
                        }
                        Ok(_) => continue,
                        Err(rusqlite::Error::SqliteFailure(error, _))
                            if matches!(
                                error.code,
                                rusqlite::ErrorCode::DatabaseBusy
                                    | rusqlite::ErrorCode::DatabaseLocked
                            ) =>
                        {
                            continue
                        }
                        Err(_) => "resyncRequired",
                    };
                    sequence = sequence.saturating_add(1);
                    if sender
                        .send(StateChangeEvent {
                            sequence,
                            reason: reason.into(),
                        })
                        .is_err()
                        || reason == "resyncRequired"
                    {
                        break;
                    }
                }
                worker_active.store(false, Ordering::Release);
            })?;
        Ok(Self {
            receiver: tokio::sync::Mutex::new(receiver),
            cancel: Mutex::new(None),
            wait_in_progress: AtomicBool::new(false),
            active,
            stop_signal,
            worker: Mutex::new(Some(worker)),
        })
    }
}

#[boltffi::export]
impl StateChangeSubscription {
    /// One pending wait per holder. Cancellation returns None and permits reuse.
    /// Foreign task cancellation also releases the pending wait.
    pub async fn next_change(&self) -> Result<Option<StateChangeEvent>, BarkError> {
        if self
            .wait_in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("next_change() called concurrently on the same holder".into());
        }
        let _guard = WaitGuard {
            wait_in_progress: &self.wait_in_progress,
            cancel: &self.cancel,
        };
        let (sender, cancelled) = oneshot::channel();
        *self.cancel.lock().unwrap() = Some(sender);
        let mut receiver = self.receiver.lock().await;
        tokio::select! {
            event = receiver.recv() => Ok(event),
            _ = cancelled => Ok(None),
        }
    }

    pub fn cancel_next_change_wait(&self) {
        let sender = self.cancel.lock().unwrap().take();
        if let Some(sender) = sender {
            let _ = sender.send(());
        }
    }

    /// Permanently stop observation. Already queued events drain before None.
    /// A pending receive with no queued event is unblocked with None.
    pub fn stop(&self) {
        self.active.store(false, Ordering::Release);
        let (lock, signal) = &*self.stop_signal;
        *lock.lock().unwrap() = true;
        signal.notify_all();
        let worker = self.worker.lock().unwrap().take();
        if let Some(worker) = worker {
            // Kotlin may resume inline on the producer thread and call stop.
            if worker.thread().id() != std::thread::current().id() {
                let _ = worker.join();
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }
}

impl Drop for StateChangeSubscription {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::runtime::TOKIO_RT;

    #[test]
    fn transient_exclusive_lock_retries_and_delivers_committed_change() {
        TOKIO_RT.block_on(async {
            let file = tempfile::NamedTempFile::new().unwrap();
            let writer = Connection::open(file.path()).unwrap();
            writer.execute_batch("CREATE TABLE test(value);").unwrap();
            let holder = StateChangeSubscription::new(file.path()).unwrap();
            assert_eq!(
                holder.next_change().await.unwrap().unwrap().reason,
                "initial"
            );
            writer
                .execute_batch("BEGIN EXCLUSIVE; INSERT INTO test VALUES(1);")
                .unwrap();
            // Cover the first 750ms poll and its 100ms busy timeout.
            tokio::time::sleep(Duration::from_millis(1100)).await;
            writer.execute_batch("COMMIT;").unwrap();
            let changed = tokio::time::timeout(Duration::from_secs(3), holder.next_change())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(changed.reason, "databaseChanged");
            assert!(holder.is_active());
            holder.stop();
        });
    }

    #[test]
    fn stop_preserves_queued_events_then_ends_stream() {
        TOKIO_RT.block_on(async {
            let file = tempfile::NamedTempFile::new().unwrap();
            Connection::open(file.path())
                .unwrap()
                .execute_batch("CREATE TABLE test(value);")
                .unwrap();
            let holder = StateChangeSubscription::new(file.path()).unwrap();
            holder.stop();
            assert!(!holder.is_active());
            assert_eq!(
                holder.next_change().await.unwrap().unwrap().reason,
                "initial"
            );
            assert!(holder.next_change().await.unwrap().is_none());
        });
    }

    #[test]
    fn committed_changes_rollback_cancel_concurrency_stop_and_reuse() {
        TOKIO_RT.block_on(async {
            let file = tempfile::NamedTempFile::new().unwrap();
            let writer = Connection::open(file.path()).unwrap();
            writer
                .execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE test(value);")
                .unwrap();
            let holder = Arc::new(StateChangeSubscription::new(file.path()).unwrap());
            let initial = holder.next_change().await.unwrap().unwrap();
            assert_eq!((initial.sequence, initial.reason.as_str()), (0, "initial"));
            writer
                .execute_batch("BEGIN; INSERT INTO test VALUES (1); ROLLBACK;")
                .unwrap();
            assert!(
                tokio::time::timeout(Duration::from_millis(900), holder.next_change())
                    .await
                    .is_err()
            );
            let pending_holder = holder.clone();
            let pending = tokio::spawn(async move { pending_holder.next_change().await });
            tokio::time::sleep(Duration::from_millis(20)).await;
            assert!(holder.next_change().await.is_err());
            holder.cancel_next_change_wait();
            assert!(pending.await.unwrap().unwrap().is_none());
            writer
                .execute_batch("INSERT INTO test VALUES (2);")
                .unwrap();
            let changed = tokio::time::timeout(Duration::from_secs(3), holder.next_change())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(
                (changed.sequence, changed.reason.as_str()),
                (1, "databaseChanged")
            );
            let pending_holder = holder.clone();
            let pending = tokio::spawn(async move { pending_holder.next_change().await });
            tokio::time::sleep(Duration::from_millis(20)).await;
            pending.abort();
            let _ = pending.await;
            holder.stop();
            assert!(!holder.is_active());
            assert!(holder.next_change().await.unwrap().is_none());
            holder.stop();
        });
    }
}
