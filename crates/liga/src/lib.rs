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
pub type AetherModuleError = liga_modules::ModuleError;
/// Official Aether name for the application context.
pub type AetherAppContext = liga_core::AppContext;
/// Official Aether name for the module event bus.
pub use liga_modules::EventBus as AetherEventBus;
/// Official Aether name for event dispatch errors.
pub use liga_modules::EventError as AetherEventError;

// Official Aether names for core configuration and validation types.
/// Official Aether name for application configuration.
pub type AetherConfig = liga_core::Config;
/// Official Aether name for configuration errors.
pub type AetherConfigError = liga_core::ConfigError;
/// Official Aether name for loading application configuration.
pub type AetherConfigLoader = liga_core::ConfigLoader;
/// Official Aether name for field-level validation errors.
pub type AetherFieldError = liga_core::FieldError;
/// Official Aether name for shared-state errors.
pub type AetherStateError = liga_core::StateError;
/// Official Aether name for the typed shared-state map.
pub type AetherStateMap = liga_core::StateMap;
/// Official Aether name for a secret value wrapper.
pub type AetherSecret<T> = liga_core::Secret<T>;
/// Official Aether name for the application shutdown signal.
pub type AetherShutdown = liga_core::Shutdown;
/// Official Aether name for validation error collections.
pub type AetherValidationErrors = liga_core::ValidationErrors;
/// Official Aether name for the validation trait.
pub use liga_core::Validate as AetherValidate;

// Official Aether names for module orchestration and events.
/// Official Aether name for contributed module integrations.
pub type AetherContributions = liga_modules::Contributions;
/// Official Aether name for event delivery reports.
pub type AetherDelivery = liga_modules::Delivery;
/// Official Aether name for event publishing errors.
pub type AetherPublishError = liga_modules::PublishError;
/// Official Aether name for an event subscription.
pub type AetherSubscription = liga_modules::Subscription;
/// Official Aether name for module orchestration errors.
pub type AetherModulesError = liga_modules::ModulesError;
/// Official Aether name for module lifecycle options.
pub type AetherLifecycleOptions = liga_modules::LifecycleOptions;
/// Official Aether name for the module collection.
pub type AetherModuleSet = liga_modules::ModuleSet;
/// Official Aether name for resolved modules.
pub type AetherResolvedModules = liga_modules::ResolvedModules;
/// Official Aether name for running modules.
pub type AetherRunningModules = liga_modules::RunningModules;

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
/// Official Aether name for task-spawn errors.
#[cfg(feature = "tasks")]
pub type AetherSpawnError = liga_tasks::SpawnError;
/// Official Aether name for task execution errors.
#[cfg(feature = "tasks")]
pub type AetherTaskError = liga_tasks::TaskError;
/// Official Aether name for task failure details.
#[cfg(feature = "tasks")]
pub type AetherTaskFailure = liga_tasks::TaskFailure;
/// Official Aether name for retry configuration.
#[cfg(feature = "tasks")]
pub type AetherRetryPolicy = liga_tasks::RetryPolicy;
/// Official Aether name for task-runner shutdown results.
#[cfg(feature = "tasks")]
pub type AetherShutdownReport = liga_tasks::ShutdownReport;
/// Official Aether name for task execution context.
#[cfg(feature = "tasks")]
pub type AetherTaskContext = liga_tasks::TaskContext;
/// Official Aether name for a spawned task handle.
#[cfg(feature = "tasks")]
pub type AetherTaskHandle<T> = liga_tasks::TaskHandle<T>;
/// Official Aether name for a task identifier.
#[cfg(feature = "tasks")]
pub type AetherTaskId = liga_tasks::TaskId;
/// Official Aether name for the task runner.
#[cfg(feature = "tasks")]
pub type AetherTaskRunner = liga_tasks::TaskRunner;
/// Official Aether name for the task-runner builder.
#[cfg(feature = "tasks")]
pub type AetherTaskRunnerBuilder = liga_tasks::TaskRunnerBuilder;
/// Official Aether name for a task specification.
#[cfg(feature = "tasks")]
pub type AetherTaskSpec = liga_tasks::TaskSpec;
/// Official Aether name for a task's status.
#[cfg(feature = "tasks")]
pub type AetherTaskStatus = liga_tasks::TaskStatus;
#[cfg(feature = "web")]
pub use liga_web as web;
/// Official Aether name for the successful API response envelope.
#[cfg(feature = "web")]
pub type AetherApiEnvelope<T> = liga_web::ApiEnvelope<T>;
/// Official Aether name for an API error body.
#[cfg(feature = "web")]
pub type AetherApiErrorBody = liga_web::ApiErrorBody;
/// Official Aether name for the API error envelope.
#[cfg(feature = "web")]
pub type AetherApiErrorEnvelope = liga_web::ApiErrorEnvelope;
/// Official Aether name for API response metadata.
#[cfg(feature = "web")]
pub type AetherApiMeta = liga_web::ApiMeta;
/// Official Aether name for HTTP API errors.
#[cfg(feature = "web")]
pub type AetherApiError = liga_web::ApiError;
/// Official Aether name for the API result type.
#[cfg(feature = "web")]
pub type AetherApiResult<T> = liga_web::ApiResult<T>;
/// Official Aether name for dependency injection extractors.
#[cfg(feature = "web")]
pub type AetherDep<T> = liga_web::Dep<T>;
/// Official Aether name for the JSON extractor.
#[cfg(feature = "web")]
pub type AetherJson<T> = liga_web::Json<T>;
/// Official Aether name for path extractors.
#[cfg(feature = "web")]
pub type AetherPath<T> = liga_web::Path<T>;
/// Official Aether name for query extractors.
#[cfg(feature = "web")]
pub type AetherQuery<T> = liga_web::Query<T>;
/// Official Aether name for validated JSON extractors.
#[cfg(feature = "web")]
pub type AetherValidJson<T> = liga_web::ValidJson<T>;
/// Official Aether name for route collections.
#[cfg(feature = "web")]
pub type AetherRoutes = liga_web::Routes;
/// Official Aether name for request ID metadata.
#[cfg(feature = "web")]
pub type AetherRequestId = liga_web::RequestId;
/// Official Aether name for HTTP server configuration.
#[cfg(feature = "web")]
pub type AetherWebServerConfig = liga_web::ServerConfig;
/// Official Aether name for HTTP server errors.
#[cfg(feature = "web")]
pub type AetherWebError = liga_web::WebError;

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
        AetherApp, AetherAppContext, AetherApplication, AetherApplicationBuilder, AetherConfig,
        AetherConfigError, AetherConfigLoader, AetherContributions, AetherDelivery, AetherError,
        AetherEventBus, AetherEventError, AetherFieldError, AetherLifecycleOptions, AetherModule,
        AetherModuleContext, AetherModuleError, AetherModuleSet, AetherModulesError,
        AetherPublishError, AetherResolvedModules, AetherRunningModules, AetherSecret,
        AetherShutdown, AetherStateError, AetherStateMap, AetherSubscription, AetherValidate,
        AetherValidationErrors, App, Application, ApplicationBuilder, Error, LigaApp,
        LigaApplication,
    };
    pub use liga_core::{AppContext, Config, Secret, Validate, ValidationErrors};
    pub use liga_modules::{EventBus, EventError, Module, ModuleContext, ModuleError};

    #[cfg(feature = "web")]
    pub use crate::{
        AetherApiEnvelope, AetherApiError, AetherApiErrorBody, AetherApiErrorEnvelope,
        AetherApiMeta, AetherApiResult, AetherDep, AetherJson, AetherPath, AetherQuery,
        AetherRequestId, AetherRoutes, AetherValidJson, AetherWebError, AetherWebServerConfig,
    };
    #[cfg(feature = "web")]
    pub use liga_web::axum::http::StatusCode;
    #[cfg(feature = "web")]
    pub use liga_web::axum::routing::{delete, get, patch, post, put};
    #[cfg(feature = "web")]
    pub use liga_web::{ApiError, ApiResult, Dep, Json, Path, Query, RequestId, Routes, ValidJson};

    #[cfg(feature = "tasks")]
    pub use crate::{
        AetherRetryPolicy, AetherShutdownReport, AetherSpawnError, AetherTaskContext,
        AetherTaskError, AetherTaskFailure, AetherTaskHandle, AetherTaskId, AetherTaskRunner,
        AetherTaskRunnerBuilder, AetherTaskSpec, AetherTaskStatus,
    };
    #[cfg(feature = "tasks")]
    pub use liga_tasks::{RetryPolicy, TaskContext, TaskError, TaskRunner, TaskSpec};

    #[cfg(feature = "ai")]
    pub use liga_ai::{
        AetherAgent, AetherAgentBuildError, AetherAgentBuilder, AetherAgentError,
        AetherAgentOutcome, AetherLimits, AetherMessage, AetherModelProvider, AetherModelRequest,
        AetherModelResponse, AetherPermission, AetherPolicy, AetherProviderError,
        AetherProviderErrorKind, AetherRole, AetherTool, AetherToolCall, AetherToolCallOutcome,
        AetherToolCallRecord, AetherToolContext, AetherToolError, AetherToolSpec, AetherUsage,
        Agent, LigaAgent, ModelProvider, Permission, Policy, Tool, ToolContext, ToolError,
        ToolSpec,
    };
}
