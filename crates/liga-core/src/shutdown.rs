//! Cloneable shutdown / cancellation signal.

use std::sync::Arc;

use tokio::sync::watch;

/// A one-way signal: once triggered it stays triggered.
///
/// Clones share the same signal. Use it for application shutdown and for
/// cancelling individual units of work.
#[derive(Clone, Debug)]
pub struct Shutdown {
    tx: Arc<watch::Sender<bool>>,
}

impl Default for Shutdown {
    fn default() -> Self {
        Self::new()
    }
}

impl Shutdown {
    /// Creates an untriggered signal.
    pub fn new() -> Self {
        let (tx, _rx) = watch::channel(false);
        Self { tx: Arc::new(tx) }
    }

    /// Triggers the signal. Idempotent.
    pub fn trigger(&self) {
        self.tx.send_replace(true);
    }

    /// Returns `true` once [`Shutdown::trigger`] has been called.
    pub fn is_triggered(&self) -> bool {
        *self.tx.borrow()
    }

    /// Resolves when the signal is triggered (immediately if it already was).
    pub async fn wait(&self) {
        let mut rx = self.tx.subscribe();
        // The sender lives as long as `self`, so this cannot fail.
        let _ = rx.wait_for(|triggered| *triggered).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn wait_resolves_after_trigger_from_a_clone() {
        let shutdown = Shutdown::new();
        let clone = shutdown.clone();
        assert!(!shutdown.is_triggered());
        let waiter = tokio::spawn(async move { clone.wait().await });
        tokio::time::sleep(Duration::from_millis(5)).await;
        shutdown.trigger();
        tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .unwrap()
            .unwrap();
        assert!(shutdown.is_triggered());
        // Already triggered: returns immediately.
        shutdown.wait().await;
    }
}
