//! # Aether
//!
//! A modular backend framework for Rust: start with a small HTTP API, grow
//! into explicit modules with events, background tasks and permissioned AI
//! agents, without rewriting the foundation.
//!
//! ```no_run
//! use liga::prelude::*;
//!
//! struct Hello;
//!
//! impl AetherModule for Hello {
//!     fn name(&self) -> &'static str { "hello" }
//!
//!     fn register(&self, ctx: &mut AetherModuleContext<'_>) -> Result<(), AetherModuleError> {
//!         ctx.contribute(Routes::new().route("/hello", get(|| async { "hello, aether" })));
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), liga::AetherError> {
//!     liga::init_tracing();
//!     AetherApplication::builder()
//!         .name("hello")
//!         .env_prefix("HELLO")          // HELLO__SERVER__ADDR=0.0.0.0:8080
//!         .module(Hello)
//!         .build()
//!         .await?
//!         .serve()
//!         .await
//! }
//! ```
//!
//! ## Features
//!
//! | feature   | default | adds |
//! |-----------|---------|------|
//! | `web`     | yes     | [`web`]: HTTP server, routes, JSON errors, middleware |
//! | `logging` | yes     | [`init_tracing`] (tracing-subscriber) |
//! | `tasks`   | no      | [`tasks`]: local task runner |
//! | `ai`      | no      | [`ai`]: providers, tools, policy, agent |
//! | `full`    | no      | all of the above |
//!
//! The core and module system are always available; nothing about HTTP or AI
//! is compiled unless its feature is enabled.

mod app;

pub use app::{App, Application, ApplicationBuilder, Error};

/// Official Aether name for Application.
pub type AetherApplication = Application;
/// Official Aether name for App.
pub type AetherApp = App;
/// Official Aether name for ApplicationBuilder.
pub type AetherApplicationBuilder = ApplicationBuilder;
/// Official Aether name for the framework's application error.
pub type AetherError = Error;
/// Official Aether name for modules.
pub use liga_modules::Module as AetherModule;
/// Official Aether name for the module registration context.
pub type AetherModuleContext<'a> = liga_modules::ModuleContext<'a>;
/// Official Aether name for module registration errors.
pub type AetherModuleError = liga_modules::AetherModuleError;
/// Official Aether name for the application context.
pub type AetherAppContext = liga_core::AppContext;
/// Official Aether name for the module event bus.
pub use liga_modules::EventBus as AetherEventBus;
/// Official Aether name for event dispatch errors.
pub use liga_modules::EventError as AetherEventError;

/// Compatibility aliases for the previous public Application name.
pub type LigaApplication = AetherApplication;
/// Compatibility alias for the previous public App name.
pub type LigaApp = AetherApp;

pub use liga_core as core;
pub use liga_modules as modules;

#[cfg(feature = "ai")]
pub use liga_ai as ai;
#[cfg(feature = "tasks")]
pub use liga_tasks as tasks;
#[cfg(feature = "web")]
pub use liga_web as web;

/// Installs a `tracing` subscriber that prints to stdout.
///
/// Filtering follows `RUST_LOG` (default `info`). Calling it twice is harmless.
#[cfg(feature = "logging")]
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

/// The names most applications need.
pub mod prelude {
    pub use crate::{
        AetherApp, AetherAppContext, AetherApplication, AetherApplicationBuilder, AetherError,
        AetherEventBus, AetherEventError, AetherModule, AetherModuleContext, AetherModuleError,
        App, Application, ApplicationBuilder, Error, LigaApp, LigaApplication,
    };
    pub use liga_core::{AppContext, Config, Secret, Validate, ValidationErrors};
    pub use liga_modules::{EventBus, EventError, Module, ModuleContext, AetherModuleError};

    #[cfg(feature = "web")]
    pub use liga_web::axum::http::StatusCode;
    #[cfg(feature = "web")]
    pub use liga_web::axum::routing::{delete, get, patch, post, put};
    #[cfg(feature = "web")]
    pub use liga_web::{ApiError, ApiResult, Dep, Json, Path, Query, RequestId, Routes, ValidJson};

    #[cfg(feature = "tasks")]
    pub use liga_tasks::{RetryPolicy, TaskContext, TaskError, TaskRunner, TaskSpec};

    #[cfg(feature = "ai")]
    pub use liga_ai::{
        AetherAgent, AetherAgentBuilder, AetherAgentError, AetherAgentOutcome, Agent, LigaAgent,
        ModelProvider, Permission, Policy, Tool, ToolContext, ToolError, ToolSpec,
    };
}
