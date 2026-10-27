//! In-memory, typed event bus.
//!
//! ## Guarantees (and non-guarantees)
//!
//! - **In memory only.** Events are lost if the process stops. This is not a
//!   queue: use a durable store for work that must not be lost.
//! - **Publisher waits.** [`EventBus::publish`] runs every handler and
//!   returns once all of them finished, failed or timed out.
//! - **Sequential, in subscription order.** Handler *n+1* starts after handler
//!   *n* finishes. Handlers of different `publish` calls may interleave.
//! - **Failures are isolated and reported.** A handler that returns an error,
//!   panics or exceeds the handler timeout does not stop the others; every
//!   failure is logged and returned in [`PublishError`].
//! - **Re-entrant.** Handlers may publish further events (no lock is held while
//!   handlers run). Avoid unbounded publish loops.
//! - Subscribing or unsubscribing during a publish affects later publishes only.

use std::any::{Any, TypeId, type_name};
use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::module::BoxFuture;

type Handler = Arc<
    dyn Fn(Arc<dyn Any + Send + Sync>) -> BoxFuture<'static, Result<(), EventError>> + Send + Sync,
>;

struct Subscriber {
    id: u64,
    handler: Handler,
}

struct Inner {
    subscribers: RwLock<HashMap<TypeId, Vec<Subscriber>>>,
    next_id: AtomicU64,
    handler_timeout: Duration,
}

/// Cloneable handle to the bus; clones share subscribers.
#[derive(Clone)]
pub struct EventBus {
    inner: Arc<Inner>,
}

impl fmt::Debug for EventBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventBus")
            .field("handler_timeout", &self.inner.handler_timeout)
            .finish_non_exhaustive()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Identifies a subscription so it can be removed with [`EventBus::unsubscribe`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Subscription {
    id: u64,
    type_id: TypeId,
}

/// Successful delivery summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delivery {
    /// Number of handlers that ran successfully (0 when nobody subscribed).
    pub handlers: usize,
}

/// Error returned by an event handler.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct EventError {
    message: Cow<'static, str>,
    #[source]
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl EventError {
    /// Creates an error with a message.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// Creates an error with a message and a cause.
    pub fn with_source(
        message: impl Into<Cow<'static, str>>,
        source: impl Into<Box<dyn StdError + Send + Sync>>,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(source.into()),
        }
    }
}

/// One or more handlers failed. The others still ran.
#[derive(Debug, thiserror::Error)]
#[error("{} of {total} handler(s) for `{event}` failed", failures.len())]
pub struct PublishError {
    /// Event type name.
    pub event: &'static str,
    /// Handlers that were invoked.
    pub total: usize,
    /// Each failure, in handler order.
    pub failures: Vec<EventError>,
}

impl EventBus {
    /// Creates a bus whose handlers time out after 30 seconds.
    pub fn new() -> Self {
        Self::with_handler_timeout(Duration::from_secs(30))
    }

    /// Creates a bus with a custom per-handler timeout.
    pub fn with_handler_timeout(handler_timeout: Duration) -> Self {
        Self {
            inner: Arc::new(Inner {
                subscribers: RwLock::new(HashMap::new()),
                next_id: AtomicU64::new(1),
                handler_timeout,
            }),
        }
    }

    /// Subscribes `handler` to events of type `E`.
    pub fn subscribe<E, F, Fut>(&self, handler: F) -> Subscription
    where
        E: Send + Sync + 'static,
        F: Fn(Arc<E>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), EventError>> + Send + 'static,
    {
        let erased: Handler =
            Arc::new(
                move |event: Arc<dyn Any + Send + Sync>| match event.downcast::<E>() {
                    Ok(event) => Box::pin(handler(event)),
                    Err(_) => Box::pin(async { Err(EventError::new("event type mismatch")) }),
                },
            );
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let type_id = TypeId::of::<E>();
        self.write().entry(type_id).or_default().push(Subscriber {
            id,
            handler: erased,
        });
        Subscription { id, type_id }
    }

    /// Removes a subscription. Returns `false` if it was already removed.
    pub fn unsubscribe(&self, subscription: Subscription) -> bool {
        let mut map = self.write();
        let Some(list) = map.get_mut(&subscription.type_id) else {
            return false;
        };
        let before = list.len();
        list.retain(|s| s.id != subscription.id);
        before != list.len()
    }

    /// Number of handlers subscribed to `E`.
    pub fn subscriber_count<E: 'static>(&self) -> usize {
        self.read().get(&TypeId::of::<E>()).map_or(0, Vec::len)
    }

    /// Delivers `event` to every subscriber of `E`. See the [module docs](self)
    /// for the exact semantics.
    pub async fn publish<E: Send + Sync + 'static>(
        &self,
        event: E,
    ) -> Result<Delivery, PublishError> {
        let handlers: Vec<Handler> = self
            .read()
            .get(&TypeId::of::<E>())
            .map(|list| list.iter().map(|s| s.handler.clone()).collect())
            .unwrap_or_default();
        let event_name = type_name::<E>();
        let event: Arc<dyn Any + Send + Sync> = Arc::new(event);
        let total = handlers.len();
        let mut failures = Vec::new();

        for (index, handler) in handlers.iter().enumerate() {
            let mut task = tokio::spawn(handler(event.clone()));
            let failure = match tokio::time::timeout(self.inner.handler_timeout, &mut task).await {
                Ok(Ok(Ok(()))) => None,
                Ok(Ok(Err(err))) => Some(err),
                Ok(Err(join)) if join.is_panic() => Some(EventError::new("event handler panicked")),
                Ok(Err(_)) => Some(EventError::new("event handler was cancelled")),
                Err(_) => {
                    task.abort();
                    Some(EventError::new(format!(
                        "event handler timed out after {:?}",
                        self.inner.handler_timeout
                    )))
                }
            };
            if let Some(err) = failure {
                tracing::warn!(event = event_name, handler = index, error = %err, "event handler failed");
                failures.push(err);
            }
        }

        if failures.is_empty() {
            Ok(Delivery { handlers: total })
        } else {
            Err(PublishError {
                event: event_name,
                total,
                failures,
            })
        }
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, HashMap<TypeId, Vec<Subscriber>>> {
        // Handlers never run under the lock, so poisoning cannot leave the map inconsistent.
        self.inner
            .subscribers
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, HashMap<TypeId, Vec<Subscriber>>> {
        self.inner
            .subscribers
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug)]
    struct Ping(u32);

    fn log() -> Arc<Mutex<Vec<String>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    #[tokio::test]
    async fn publishing_without_subscribers_succeeds() {
        let bus = EventBus::new();
        assert_eq!(
            bus.publish(Ping(1)).await.unwrap(),
            Delivery { handlers: 0 }
        );
    }

    #[tokio::test]
    async fn handlers_run_sequentially_in_subscription_order() {
        let bus = EventBus::new();
        let seen = log();
        for name in ["a", "b", "c"] {
            let seen = seen.clone();
            bus.subscribe(move |ping: Arc<Ping>| {
                let seen = seen.clone();
                async move {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    seen.lock().unwrap().push(format!("{name}{}", ping.0));
                    Ok(())
                }
            });
        }
        assert_eq!(bus.publish(Ping(7)).await.unwrap().handlers, 3);
        assert_eq!(*seen.lock().unwrap(), ["a7", "b7", "c7"]);
    }

    #[tokio::test]
    async fn failures_are_isolated_and_reported() {
        let bus = EventBus::with_handler_timeout(Duration::from_millis(50));
        let seen = log();
        bus.subscribe(|_: Arc<Ping>| async { Err(EventError::new("boom")) });
        bus.subscribe(|_: Arc<Ping>| async { panic!("handler bug") });
        bus.subscribe(|_: Arc<Ping>| async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok(())
        });
        let after = seen.clone();
        bus.subscribe(move |_: Arc<Ping>| {
            let after = after.clone();
            async move {
                after.lock().unwrap().push("ran".into());
                Ok(())
            }
        });
        let err = bus.publish(Ping(1)).await.unwrap_err();
        assert_eq!(err.total, 4);
        assert_eq!(err.failures.len(), 3);
        assert_eq!(err.failures[0].to_string(), "boom");
        assert!(err.failures[1].to_string().contains("panicked"));
        assert!(err.failures[2].to_string().contains("timed out"));
        assert_eq!(*seen.lock().unwrap(), ["ran"]);
    }

    #[tokio::test]
    async fn handlers_may_publish_reentrantly() {
        #[derive(Debug)]
        struct Pong;
        let bus = EventBus::new();
        let seen = log();
        let inner_bus = bus.clone();
        bus.subscribe(move |_: Arc<Ping>| {
            let bus = inner_bus.clone();
            async move {
                bus.publish(Pong)
                    .await
                    .map_err(|e| EventError::with_source("nested publish failed", e))?;
                Ok(())
            }
        });
        let pongs = seen.clone();
        bus.subscribe(move |_: Arc<Pong>| {
            let pongs = pongs.clone();
            async move {
                pongs.lock().unwrap().push("pong".into());
                Ok(())
            }
        });
        bus.publish(Ping(1)).await.unwrap();
        assert_eq!(*seen.lock().unwrap(), ["pong"]);
    }

    #[tokio::test]
    async fn unsubscribe_removes_only_that_handler() {
        let bus = EventBus::new();
        let a = bus.subscribe(|_: Arc<Ping>| async { Ok(()) });
        bus.subscribe(|_: Arc<Ping>| async { Ok(()) });
        assert_eq!(bus.subscriber_count::<Ping>(), 2);
        assert!(bus.unsubscribe(a));
        assert!(!bus.unsubscribe(a));
        assert_eq!(bus.publish(Ping(1)).await.unwrap().handlers, 1);
    }
}
