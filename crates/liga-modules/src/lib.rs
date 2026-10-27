//! # liga-modules
//!
//! Explicit, validated composition of an application from [`Module`]s.
//!
//! 1. Collect modules in a [`ModuleSet`].
//! 2. [`ModuleSet::resolve`] rejects duplicate names, missing dependencies and
//!    cycles, and orders modules so dependencies come first.
//! 3. [`ResolvedModules::register`] runs each module's synchronous
//!    `register` hook (provide shared state, contribute routes, subscribe to
//!    events). No I/O happens here.
//! 4. [`ResolvedModules::start`] runs `start` hooks in dependency order. If one
//!    fails, the modules already started are shut down in reverse order.
//! 5. [`RunningModules::shutdown`] runs `shutdown` hooks in reverse order.
//!
//! This crate does not depend on HTTP. Integrations such as `liga-web` read
//! the values modules [contribute](ModuleContext::contribute).

mod context;
pub mod events;
mod module;
mod set;

pub use context::{Contributions, ModuleContext};
pub use events::{Delivery, EventBus, EventError, PublishError, Subscription};
pub use module::{Module, ModuleError};
pub use set::{LifecycleOptions, ModuleSet, ModulesError, ResolvedModules, RunningModules};
