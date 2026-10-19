use once_cell::sync::Lazy;
use std::future::Future;
use tokio::runtime::Runtime;

/// Global multi-threaded Tokio runtime for all Bark async work
pub static TOKIO_RT: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("bark-ffi-runtime")
        .build()
        .expect("Failed to create Tokio runtime")
});

/// Run an async closure on the runtime
///
/// Foreign cancellation drops the join handle; work already submitted to Tokio
/// continues. A cancelled payment wait does not imply payment rollback.
pub(crate) async fn run_async<F, T>(future: F) -> T
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    TOKIO_RT.spawn(future).await.expect("Task panicked")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelled_foreign_wait_keeps_submitted_work_running() {
        TOKIO_RT.block_on(async {
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let (finish_tx, finish_rx) = tokio::sync::oneshot::channel();
            let (done_tx, done_rx) = tokio::sync::oneshot::channel();
            let task = tokio::spawn(run_async(async move {
                started_tx.send(()).unwrap();
                finish_rx.await.unwrap();
                done_tx.send(()).unwrap();
            }));
            started_rx.await.unwrap();
            task.abort();
            let _ = task.await;
            finish_tx.send(()).unwrap();
            tokio::time::timeout(std::time::Duration::from_secs(5), done_rx)
                .await
                .unwrap()
                .unwrap();
        });
    }
}
