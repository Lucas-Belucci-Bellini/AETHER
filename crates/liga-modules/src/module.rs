use std::borrow::Cow;
use std::error::Error as StdError;
use std::future::Future;
use std::pin::Pin;

use liga_core::{AppContext, ConfigError, StateError};

use crate::context::ModuleContext;

pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A unit of application composition.
///
/// Every hook has a no-op default; implement only what the module needs.
/// `start` and `shutdown` can be written as `async fn` in the `impl` block.
///
/// ```
/// use liga_core::AppContext;
/// use liga_modules::{Module, ModuleContext, ModuleError};
///
/// struct Greeter;
///
/// impl Module for Greeter {
///     fn name(&self) -> &'static str { "greeter" }
///
///     fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> {
///         ctx.provide(String::from("hello"))?;
///         Ok(())
///     }
///
///     async fn start(&self, _ctx: &AppContext) -> Result<(), ModuleError> {
///         Ok(())
///     }
/// }
/// ```
pub trait Module: Send + Sync + 'static {
    /// Unique module name. Other modules refer to it in [`Module::dependencies`].
    fn name(&self) -> &'static str;

    /// Names of modules that must be registered and started before this one.
    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }

    /// Synchronous registration: read configuration, provide shared state,
    /// contribute integration values and subscribe to events. Do not do I/O here.
    fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> {
        let _ = ctx;
        Ok(())
    }

    /// Acquires resources (connections, background workers, warm-up).
    fn start(&self, ctx: &AppContext) -> impl Future<Output = Result<(), ModuleError>> + Send {
        let _ = ctx;
        async { Ok(()) }
    }

    /// Releases resources. Called in reverse start order.
    fn shutdown(&self, ctx: &AppContext) -> impl Future<Output = Result<(), ModuleError>> + Send {
        let _ = ctx;
        async { Ok(()) }
    }
}

/// Object-safe form of [`Module`], used internally to store heterogeneous modules.
pub(crate) trait DynModule: Send + Sync {
    fn name(&self) -> &'static str;
    fn dependencies(&self) -> &'static [&'static str];
    fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError>;
    fn start<'a>(&'a self, ctx: &'a AppContext) -> BoxFuture<'a, Result<(), ModuleError>>;
    fn shutdown<'a>(&'a self, ctx: &'a AppContext) -> BoxFuture<'a, Result<(), ModuleError>>;
}

impl<M: Module> DynModule for M {
    fn name(&self) -> &'static str {
        Module::name(self)
    }

    fn dependencies(&self) -> &'static [&'static str] {
        Module::dependencies(self)
    }

    fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> {
        Module::register(self, ctx)
    }

    fn start<'a>(&'a self, ctx: &'a AppContext) -> BoxFuture<'a, Result<(), ModuleError>> {
        Box::pin(Module::start(self, ctx))
    }

    fn shutdown<'a>(&'a self, ctx: &'a AppContext) -> BoxFuture<'a, Result<(), ModuleError>> {
        Box::pin(Module::shutdown(self, ctx))
    }
}

/// Error returned by a module hook.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ModuleError {
    message: Cow<'static, str>,
    #[source]
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl ModuleError {
    /// Creates an error with a message.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// Creates an error with a message and an underlying cause.
    pub fn with_source(
        message: impl Into<Cow<'static, str>>,
        source: impl Into<Box<dyn StdError + Send + Sync>>,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    /// The error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<ConfigError> for ModuleError {
    fn from(err: ConfigError) -> Self {
        Self::with_source(err.to_string(), err)
    }
}

impl From<StateError> for ModuleError {
    fn from(err: StateError) -> Self {
        Self::with_source(err.to_string(), err)
    }
}
