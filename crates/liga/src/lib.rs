//! Liga — modular backend framework for Rust.
//! LigaApplication and LigaApp are framework-branded names for the existing
//! Application and App types. The original names remain available.
//!
//! Example:
//! use liga::prelude::*;
//! LigaApplication::builder().name("hello");
//!
//! Features: web (default), logging (default), tasks, ai and full.

mod app;

pub use app::{App, Application, ApplicationBuilder, Error};

/// Framework-branded name for Application.
pub type LigaApplication = Application;
/// Framework-branded name for App.
pub type LigaApp = App;

pub use liga_core as core;
pub use liga_modules as modules;

#[cfg(feature = "ai")]
pub use liga_ai as ai;
#[cfg(feature = "tasks")]
pub use liga_tasks as tasks;
#[cfg(feature = "web")]
pub use liga_web as web;

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
    pub use crate::{App, Application, ApplicationBuilder, Error, LigaApp, LigaApplication};
    pub use liga_core::{AppContext, Config, Secret, Validate, ValidationErrors};
    pub use liga_modules::{EventBus, EventError, Module, ModuleContext, ModuleError};

    #[cfg(feature = "web")]
    pub use liga_web::axum::http::StatusCode;
    #[cfg(feature = "web")]
    pub use liga_web::axum::routing::{delete, get, patch, post, put};
    #[cfg(feature = "web")]
    pub use liga_web::{ApiError, ApiResult, Dep, Json, Path, Query, RequestId, Routes, ValidJson};

    #[cfg(feature = "tasks")]
    pub use liga_tasks::{RetryPolicy, TaskContext, TaskError, TaskRunner, TaskSpec};

    #[cfg(feature = "ai")]
    pub use liga_ai::{Agent, LigaAgent, ModelProvider, Permission, Policy, Tool, ToolContext, ToolError, ToolSpec};
}
