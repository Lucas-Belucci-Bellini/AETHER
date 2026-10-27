//! # liga-core
//!
//! The small, stable centre of the Liga framework. It knows nothing about
//! HTTP, AI providers or task runners; every other Liga crate builds on it.
//!
//! - [`Config`] / [`ConfigLoader`]: layered, typed configuration
//!   (defaults < files < environment) validated at startup.
//! - [`AppContext`]: cheap-to-clone handle to the application name,
//!   configuration, typed shared state and shutdown signal.
//! - [`Shutdown`]: a cloneable, awaitable shutdown/cancellation signal.
//! - [`Secret`]: a wrapper that never prints or serializes its contents.
//! - [`Validate`]: the validation contract used for configuration and inputs.

mod coerce;
pub mod config;
pub mod context;
pub mod secret;
pub mod shutdown;
pub mod validate;

pub use config::{Config, ConfigError, ConfigLoader};
pub use context::{AppContext, StateError, StateMap};
pub use secret::Secret;
pub use shutdown::Shutdown;
pub use validate::{FieldError, Validate, ValidationErrors};
