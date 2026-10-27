//! Application context and typed shared state.

use std::any::{Any, TypeId, type_name};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::config::Config;
use crate::shutdown::Shutdown;

/// Errors when reading or writing typed shared state.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum StateError {
    /// Nothing of this type was provided.
    #[error("no shared state of type `{type_name}` was provided by any module")]
    Missing {
        /// Requested type.
        type_name: &'static str,
    },
    /// The type was provided twice.
    #[error("shared state of type `{type_name}` was provided more than once")]
    Duplicate {
        /// Duplicated type.
        type_name: &'static str,
    },
}

struct Entry {
    value: Arc<dyn Any + Send + Sync>,
    type_name: &'static str,
}

/// Type-indexed map holding at most one value per type.
#[derive(Default)]
pub struct StateMap {
    entries: HashMap<TypeId, Entry>,
}

impl fmt::Debug for StateMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set()
            .entries(self.entries.values().map(|e| e.type_name))
            .finish()
    }
}

impl StateMap {
    /// Creates an empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a value; fails if a value of the same type already exists.
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) -> Result<(), StateError> {
        self.insert_arc(Arc::new(value))
    }

    /// Inserts a shared value; fails if a value of the same type already exists.
    pub fn insert_arc<T: Send + Sync + 'static>(
        &mut self,
        value: Arc<T>,
    ) -> Result<(), StateError> {
        let id = TypeId::of::<T>();
        if self.entries.contains_key(&id) {
            return Err(StateError::Duplicate {
                type_name: type_name::<T>(),
            });
        }
        self.entries.insert(
            id,
            Entry {
                value,
                type_name: type_name::<T>(),
            },
        );
        Ok(())
    }

    /// Returns the value of type `T`, if present.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.entries
            .get(&TypeId::of::<T>())
            .and_then(|entry| entry.value.clone().downcast::<T>().ok())
    }

    /// Returns `true` if a value of type `T` is present.
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.entries.contains_key(&TypeId::of::<T>())
    }

    /// Number of stored values.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

struct Inner {
    name: String,
    config: Config,
    state: StateMap,
    shutdown: Shutdown,
}

/// Cheap-to-clone handle shared by modules, handlers, tasks and agents.
///
/// State is frozen once the application is built: modules provide values
/// during registration, everything afterwards only reads.
#[derive(Clone)]
pub struct AppContext {
    inner: Arc<Inner>,
}

impl fmt::Debug for AppContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppContext")
            .field("name", &self.inner.name)
            .field("state", &self.inner.state)
            .field("shutdown_triggered", &self.inner.shutdown.is_triggered())
            .finish()
    }
}

impl AppContext {
    /// Assembles a context. Usually called by the application builder.
    pub fn new(
        name: impl Into<String>,
        config: Config,
        state: StateMap,
        shutdown: Shutdown,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                name: name.into(),
                config,
                state,
                shutdown,
            }),
        }
    }

    /// Application name.
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Merged configuration.
    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    /// Application-wide shutdown signal.
    pub fn shutdown(&self) -> &Shutdown {
        &self.inner.shutdown
    }

    /// Shared state of type `T`, if provided.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.inner.state.get::<T>()
    }

    /// Shared state of type `T`, or [`StateError::Missing`].
    pub fn require<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, StateError> {
        self.get::<T>().ok_or(StateError::Missing {
            type_name: type_name::<T>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Counter(u32);

    #[test]
    fn state_is_typed_and_unique() {
        let mut state = StateMap::new();
        state.insert(Counter(1)).unwrap();
        assert_eq!(
            state.insert(Counter(2)),
            Err(StateError::Duplicate {
                type_name: type_name::<Counter>()
            })
        );
        let ctx = AppContext::new("test", Config::default(), state, Shutdown::new());
        assert_eq!(*ctx.require::<Counter>().unwrap(), Counter(1));
        assert!(matches!(
            ctx.require::<String>(),
            Err(StateError::Missing { .. })
        ));
    }
}
